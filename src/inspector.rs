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

/// A named group of inspectable entries.
#[derive(Clone, Debug)]
pub struct InspectSection {
    pub id: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
    pub empty: &'static str,
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

/// The inspector panel: header + one collapsible, inspectable list per section.
pub fn inspector_panel(t: &Theme, sections: Vec<InspectSection>) -> impl IntoElement {
    let mut panel = div().w(px(300.0)).flex_shrink_0().bg(t.background).border_l_1().border_color(t.border)
        .flex().flex_col().overflow_y_scrollbar()
        .child(div().px_4().py_3().border_b_1().border_color(t.border)
            .child(Label::new("Workspace context").font_weight(FontWeight::SEMIBOLD).text_size(px(13.0)))
            .child(Label::new("Inspector").text_size(px(11.0)).text_color(t.muted_foreground)));
    for section in sections {
        panel = panel.child(inspect_section(t, section));
    }
    panel
}

/// One inspectable section (header + expandable rows). Click a row header to
/// expand its detail in place; rows with a URL get an "open in browser" button.
pub fn inspect_section(t: &Theme, section: InspectSection) -> Div {
    let mut sec = div().px_4().py_3().border_b_1().border_color(t.border).flex().flex_col().gap_1()
        .child(div().flex().items_center().justify_between()
            .child(div().flex().items_center().gap_2()
                .child(Label::new(section.icon).text_size(px(13.0)).text_color(t.primary))
                .child(Label::new(section.label).font_weight(FontWeight::SEMIBOLD).text_size(px(12.0))))
            .child(Label::new(format!("{}", section.items.len())).text_size(px(10.5)).text_color(t.muted_foreground)));
    if section.items.is_empty() {
        sec = sec.child(Label::new(section.empty).text_size(px(11.0)).text_color(t.muted_foreground));
    }
    for item in section.items {
        let key = format!("{}:{}", section.id, item.id);
        let open = is_open(&key);
        let key_toggle = key.clone();
        let mut row = div().flex().flex_col().gap_1().p_2().rounded_md().bg(t.muted)
            .child(div().flex().items_center().justify_between().gap_2()
                .child(Button::new(ElementId::Name(SharedString::from(format!("insp-h-{key}"))))
                    .ghost().small().flex_1().justify_start()
                    .on_click(move |_, window, _cx| { toggle(&key_toggle); window.refresh(); })
                    .child(div().flex().items_center().gap_2().overflow_hidden()
                        .child(Label::new(if open { "▾" } else { "▸" }).text_size(px(10.0)).text_color(t.muted_foreground))
                        .child(Label::new(item.title.clone()).text_size(px(12.0)).truncate())))
                .children(item.url.as_ref().map(|u| {
                    let u = u.clone();
                    Button::new(ElementId::Name(SharedString::from(format!("insp-open-{key}"))))
                        .label("↗").ghost().xsmall()
                        .on_click(move |_, _, _| open_url(&u))
                })));
        if !item.subtitle.is_empty() {
            row = row.child(Label::new(item.subtitle.clone()).text_size(px(10.5)).text_color(t.muted_foreground).truncate());
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
