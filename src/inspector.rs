//! Shared right-side context inspector. Every section lists the *actual* items
//! the agent produced (delegations, commands, artifacts, web readings, sources,
//! links) and each row is inspectable: click to expand its detail in place, and
//! rows carrying a URL get an "open in browser" affordance.

use gpui_kit::component::{
    button::{Button, ButtonVariants},
    label::Label,
    scroll::ScrollableElement,
    Sizable, Theme,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    /// Expanded inspector row keys, persisted across re-renders on the UI thread.
    static EXPANDED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn is_open(key: &str) -> bool {
    EXPANDED.with(|e| e.borrow().contains(key))
}

fn toggle(key: &str) {
    EXPANDED.with(|e| {
        let mut b = e.borrow_mut();
        if b.contains(key) { b.remove(key); } else { b.insert(key.to_string()); }
    });
}

/// One inspectable entry in a section.
#[derive(Clone, Debug)]
pub struct InspectItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub detail: String,
    pub url: Option<String>,
}

/// A named group of inspectable entries. `summary` is the line the row shows
/// while it holds nothing, worded the way the PWA inspector words it.
#[derive(Clone, Debug)]
pub struct InspectSection {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub summary: String,
    pub items: Vec<InspectItem>,
}

/// Open a URL in the OS default browser without flashing a console window.
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}

/// Pull http(s) links out of free text so they can be listed in the inspector.
pub fn extract_links(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split_whitespace() {
        let w = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != ':' && c != '.' && c != '-' && c != '_' && c != '?' && c != '=' && c != '&' && c != '%' && c != '#' && c != '~');
        if (w.starts_with("http://") || w.starts_with("https://")) && w.len() > 10 && !out.contains(&w.to_string()) {
            out.push(w.to_string());
        }
    }
    out
}

/// The calls that hold an OS process open: a command the run is still waiting
/// on. Once it returns there is no process left, only an action the timeline
/// already shows, so the inspector's Processes section drops it.
pub const COMMAND_TOOLS: [&str; 4] = ["run_shell", "run_python", "git_status", "git_diff"];

/// The tools every run has switched on from its first token — read, write,
/// search, run. Nobody enabled them, so listing them as "skills" says nothing;
/// the Skills section is for what sits on top of these.
pub const DEFAULT_TOOLS: [&str; 6] =
    ["read_file", "write_file", "edit_file", "run_shell", "grep", "semantic_search"];

pub fn is_default_tool(kind: &str) -> bool {
    DEFAULT_TOOLS.contains(&kind)
}

pub fn is_command_tool(kind: &str) -> bool {
    COMMAND_TOOLS.contains(&kind)
}

/// The words `describe_call` gives a tool when it has no arguments yet:
/// `read_file` → "Read file". Derived rather than written out again, so
/// rewording a call cannot leave the title matching below matching nothing.
fn phrase_of(kind: &str) -> String {
    crate::attention::describe_call(kind, &serde_json::Value::Null).1.trim_end().to_string()
}

/// The native shells spell a call with its tool-table badge in front —
/// `"CMD · Run command ls"` — while the loop's own record and the PWA keep the
/// phrase alone. Both have to reach the same row, so the badge comes off before
/// the phrase is compared. Only an all-caps token counts as a badge: a command
/// string may contain ` · ` itself, and cutting that would rename the call.
fn strip_badge(title: &str) -> &str {
    match title.split_once(" · ") {
        Some((head, rest))
            if !head.is_empty() && head.chars().all(|c| c.is_uppercase() || c == '_') =>
        {
            rest
        }
        _ => title,
    }
}

fn titles_start_with(kind: &str, title: &str) -> bool {
    let phrase = phrase_of(kind);
    let title = strip_badge(title);
    !title.is_empty() && !phrase.is_empty() && title.starts_with(phrase.as_str())
}

/// The same decision for a surface whose history kept only the human title of
/// the call, which is what both shells store.
pub fn title_is_default(title: &str) -> bool {
    DEFAULT_TOOLS.iter().any(|k| titles_start_with(k, title))
}

pub fn title_is_command(title: &str) -> bool {
    COMMAND_TOOLS.iter().any(|k| titles_start_with(k, title))
}

/// One inspectable section (header + expandable rows). Click a row header to
/// expand its detail in place; rows with a URL get an "open in browser" button.
pub fn inspect_section(t: &Theme, section: InspectSection) -> impl IntoElement {
    // The section id namespaces every label inside it. `Label::new` is
    // `#[track_caller]`, so all six sections built by one app would otherwise
    // hand the a11y tree the same call-site id and the duplicate would abort a
    // debug build or silently drop the row from the tree in a release build.
    let id = ElementId::Name(SharedString::from(format!("insp-sec-{}", section.id)));
    let count = section.items.len();
    let group_key = format!("insp-{}", section.id);
    let group_open = is_open(&group_key);
    let group_toggle = group_key.clone();
    let hover = crate::theme::hover();
    // `.context-item`: an icon column, then the section name and what it holds,
    // hairlined below. The six rows read as one list under a single caption,
    // the way the PWA inspector is built, and a row with items opens them here.
    let head = div().id(ElementId::Name(SharedString::from(format!("insp-h-{group_key}"))))
        .flex().items_center().gap_3().py_3().border_b_1().border_color(t.border)
        .cursor_pointer().hover(|d| d.bg(hover))
        .child(div().w(px(20.0)).flex_shrink_0().flex().justify_center()
            .child(Label::new(section.icon).text_size(px(16.0))))
        .child(div().flex().flex_col().gap_1().flex_1().overflow_hidden()
            .child(Label::new(section.label).text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD).text_color(t.foreground))
            .child(Label::new(if count == 0 {
                section.summary.clone()
            } else {
                format!("{} item{}", count, if count == 1 { "" } else { "s" })
            }).text_size(px(12.0)).text_color(t.muted_foreground)))
        .when(count > 0, |d| d.child(Label::new(if group_open { "▾" } else { "▸" })
            .text_size(px(10.0)).text_color(t.muted_foreground)))
        .on_click(move |_, window, _cx| { toggle(&group_toggle); window.refresh(); });
    let mut sec = div().id(id).flex().flex_col().gap_1().child(head);
    let items = if group_open { section.items } else { Vec::new() };
    for item in items {
        let key = format!("{}:{}", section.id, item.id);
        let open = is_open(&key);
        let key_toggle = key.clone();
        // `.context-item`: a hairline under each row, not a box around it. The
        // path and command strings keep their full length — a row exists to say
        // which file was touched, so it wraps instead of being cut.
        let head = div().flex().items_center().gap_2()
            .child(Label::new(if open { "▾" } else { "▸" }).text_size(px(10.0)).text_color(t.muted_foreground))
            .child(Label::new(item.title.clone()).text_size(px(12.0)).font_weight(FontWeight::MEDIUM));
        let open_btn = item.url.as_ref().map(|u| {
            let u = u.clone();
            Button::new(ElementId::Name(SharedString::from(format!("insp-open-{key}"))))
                .label("↗").ghost().xsmall().flex_shrink_0()
                .on_click(move |_, _, _| open_url(&u))
        });
        let mut row = div().id(ElementId::Name(SharedString::from(format!("insp-row-{key}"))))
            .flex().flex_col().gap_1().py_2().border_b_1().border_color(t.border)
            .child(
                div().flex().items_center().justify_between().gap_2()
                    .child(
                        Button::new(ElementId::Name(SharedString::from(format!("insp-h-{key}"))))
                            .ghost().small().flex_1().justify_start()
                            .on_click(move |_, window, _cx| { toggle(&key_toggle); window.refresh(); })
                            .child(head),
                    )
                    .children(open_btn),
            );
        if !item.subtitle.is_empty() {
            row = row.child(div().pl_4().child(
                Label::new(item.subtitle.clone()).font_family("monospace").text_size(px(11.0))
                    .text_color(t.muted_foreground)));
        }
        if open && !item.detail.is_empty() {
            row = row.child(div().border_t_1().border_color(t.border).pt_1()
                .font_family("monospace").max_h(px(140.0)).overflow_y_scrollbar()
                .child(Label::new(item.detail.clone()).text_size(px(10.5)).text_color(t.foreground)));
        }
        if open {
            if let Some(u) = &item.url {
                row = row.child(Label::new(u.clone()).text_size(px(10.0)).text_color(t.link).truncate());
            }
        }
        sec = sec.child(row);
    }
    sec
}

#[cfg(test)]
mod tests {
    // Named imports, not `use super::*`: this module's glob reaches the
    // `gpui_kit::test` attribute macro (re-exported from `gpui`), which
    // `#[test]` then resolves to instead of the built-in harness.
    use crate::inspector::{
        is_command_tool, is_default_tool, strip_badge, title_is_command, title_is_default,
        DEFAULT_TOOLS,
    };

    #[test]
    fn default_tools_are_hidden_from_skills() {
        for kind in ["read_file", "write_file", "edit_file", "run_shell", "grep", "semantic_search"] {
            assert!(is_default_tool(kind), "{kind} is always on, not a skill");
        }
        for kind in ["web_task", "coworker_task", "summarise", "run_python"] {
            assert!(!is_default_tool(kind), "{kind} is something the user switched on");
        }
    }

    #[test]
    fn titles_match_their_tool() {
        // The titles are what describe_call actually produces.
        for kind in DEFAULT_TOOLS {
            let title = crate::attention::describe_call(kind, &serde_json::json!({"path": "a/b.rs"})).1;
            assert!(title_is_default(&title), "{kind} titled {title:?} must read as default");
        }
        assert!(title_is_command("Run command echo hi"));
        assert!(title_is_command("Repository status"));
        assert!(!title_is_command("Read file a/b.rs"));
        assert!(!title_is_default("Web research task"));
        assert!(!title_is_default(""));
    }

    #[test]
    fn the_native_badge_spelling_resolves_to_the_same_row() {
        // The desktop shells store the call as `"CMD · Run command ls"`; the
        // loop's own record and the PWA store the phrase alone. Same call, so
        // the same section — otherwise a command hides in Skills and never
        // reaches Processes.
        assert!(title_is_default("CMD · Run command ls -la"));
        assert!(title_is_command("CMD · Run command ls -la"));
        assert!(title_is_default("READ · Read file src/main.rs"));
        assert!(title_is_default("EDIT · Write file src/main.rs"));
        assert!(title_is_default("FIND · Search code todo"));
        assert!(title_is_command("GIT · Repository status"));
        assert!(!title_is_default("WEB · Web research task"));
        // Only an all-caps token is a badge. A command that contains the
        // separator keeps its own words rather than being renamed by a split.
        assert_eq!(strip_badge("Run command echo a · Run file b"), "Run command echo a · Run file b");
    }

    #[test]
    fn a_command_is_not_a_skill_and_a_read_is_not_a_process() {
        assert!(is_command_tool("run_shell"));
        assert!(!is_command_tool("read_file"));
        assert!(is_default_tool("run_shell"));
    }
}
