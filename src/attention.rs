//! Shared "My Attention" rail: the badge that counts what still waits for the
//! user, and the list of commands the user has already answered.
//!
//! Both Fennec shells gate a tool call through the same four-answer question, so
//! the answer words, the risk colors, and the row layout live here once instead
//! of drifting apart in two `gui_main.rs` files. Apps still own the gathering:
//! each hands over the answered approvals its own backend reported.

use gpui_kit::component::{
    Disableable,
    button::{Button, ButtonVariants},
    label::Label,
    Sizable, Theme,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::Value;

/// `fennec_core/loop.py:_APPROVAL_OPTIONS`. The answers a shell may send, and
/// the only four the loop accepts.
pub const APPROVAL_DECISIONS: [&str; 4] = ["allow_once", "allow_always", "reject_once", "reject_always"];

/// What each of the loop's options says on a button and in the answered list.
pub fn decision_label(decision: &str) -> &'static str {
    match decision {
        "allow_once" => "Allow once",
        "allow_always" => "Allow every time",
        "reject_once" => "Reject",
        "reject_always" => "Reject and stop asking",
        _ => "",
    }
}

/// One answered approval, carrying where it came from so the row can jump there.
pub struct ApprovedCommand {
    pub task_id: String,
    pub task_title: String,
    /// The gated call itself — a command line, or a tool name plus its target.
    pub command: String,
    /// `"high"` / `"medium"` / `"low"` / `"unknown"`, as the gate table reported it.
    pub risk: String,
    /// The raw decision key, resolved to its label when the row renders.
    pub decision: String,
}

/// The risks the gate table reports. A word outside these three — a renamed
/// tier, a step from a backend that has no table — is `Unknown`, which reads as
/// "nobody classified this" instead of borrowing a colour nobody chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Risk {
    High,
    Medium,
    Low,
    Unknown,
}

pub fn parse_risk(risk: &str) -> Risk {
    match risk {
        "high" => Risk::High,
        "medium" => Risk::Medium,
        "low" => Risk::Low,
        _ => Risk::Unknown,
    }
}

pub fn risk_color(t: &Theme, risk: &str) -> Hsla {
    match parse_risk(risk) {
        Risk::High => t.red,
        Risk::Medium => t.yellow,
        Risk::Low => t.success,
        Risk::Unknown => t.muted_foreground,
    }
}

/// The risk as the gate table spelled it, or `unknown` for anything else — a
/// renamed tier included. A card never guesses at a risk it was not told.
pub fn risk_word(s: Option<&str>) -> String {
    let lowered = s.unwrap_or("").trim().to_lowercase();
    match parse_risk(&lowered) {
        Risk::Unknown => "unknown".into(),
        _ => lowered,
    }
}

/// Text off a step, bounded so an oversized or hostile payload cannot grow a
/// card. Same limits the PWA puts on the same fields.
pub fn clip(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() > max { t.chars().take(max).collect() } else { t.to_string() }
}

/// What a gated call is, in the two words a row shows: the tool's own table
/// names the category, so a shell does not invent its own colour-coding.
pub fn describe_call(name: &str, args: &Value) -> (String, String) {
    let focus = |keys: &[&str]| -> String {
        for k in keys {
            if let Some(s) = args.get(*k).and_then(|v| v.as_str()) {
                return s.to_string();
            }
        }
        String::new()
    };
    match name {
        "list_dir" => ("LS".into(), format!("Inspect repository {}", focus(&["path", "dir"]))),
        "read_file" => ("READ".into(), format!("Read file {}", focus(&["path", "file"]))),
        "write_file" => ("EDIT".into(), format!("Write file {}", focus(&["path"]))),
        "edit_file" => ("EDIT".into(), format!("Edit file {}", focus(&["path"]))),
        "grep" => ("FIND".into(), format!("Search code {}", focus(&["pattern", "query"]))),
        "semantic_search" => ("FIND".into(), format!("Semantic search {}", focus(&["query"]))),
        "run_python" => ("TEST".into(), format!("Run python {}", focus(&["code"]).chars().take(48).collect::<String>())),
        "run_shell" => ("CMD".into(), format!("Run command {}", focus(&["cmd", "command"]))),
        "git_status" => ("GIT".into(), "Repository status".into()),
        "git_diff" => ("DIFF".into(), "Review diff".into()),
        "web_task" => ("WEB".into(), "Web research task".into()),
        "coworker_task" => ("PEER".into(), "Delegate to OpenBot coworker".into()),
        "summarise" => ("SUM".into(), "Summarise the run".into()),
        other => (other.to_uppercase(), other.into()),
    }
}

/// The verb a delivered file is described with — only the tools that put bytes
/// on disk deliver one. A read names a path but changes nothing, so it delivers
/// nothing, and every shell keys its file card on this same table.
pub fn delivery_verb(name: &str) -> Option<&'static str> {
    match name {
        "write_file" => Some("created"),
        "edit_file" => Some("updated"),
        _ => None,
    }
}

/// The same verb read off the display title ("Write file src/x.py") instead of
/// the tool name. The desktop shell keys its file cards on the title, so both
/// spellings have to resolve to one verb or the two shells would describe the
/// same run differently.
pub fn delivery_verb_from_title(title: &str) -> Option<&'static str> {
    match title.split_whitespace().next()?.to_ascii_lowercase().as_str() {
        "write" => Some("created"),
        "edit" => Some("updated"),
        _ => None,
    }
}

/// What the run changed, appended to its last answer once it is done: one line
/// per file it put on disk, with the verb the call used. Paths stay as the call
/// wrote them (repo-relative), because that is what the user types to open one.
/// Empty when nothing was delivered — a read-only run has no files to list.
pub fn delivery_summary(items: &[(&str, &str)]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let mut seen: Vec<&str> = Vec::new();
    let mut lines = String::new();
    for (path, verb) in items {
        if seen.contains(path) {
            continue;
        }
        seen.push(path);
        lines.push_str(&format!("- `{path}` — {verb}\n"));
    }
    format!("**Delivered {} file{}**\n\n{}", seen.len(), if seen.len() == 1 { "" } else { "s" }, lines.trim_end())
}

/// The call itself, as one compact line under the title.
pub fn args_line(args: &Value) -> String {
    let compact = args.to_string();
    if compact.chars().count() > 240 { compact.chars().take(240).collect() } else { compact }
}

/// The file a write is about, read from the call's own arguments.
pub fn path_of(args: &Value) -> Option<String> {
    ["path", "file"].iter()
        .find_map(|k| args.get(*k).and_then(|v| v.as_str()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// The count chip on the nav item: nothing at all when no task waits, so the
/// row does not carry a zero.
pub fn attention_badge(t: &Theme, count: usize) -> Option<Label> {
    (count > 0).then(|| {
        Label::new(format!("{count}"))
            .text_size(px(10.5))
            .text_color(t.yellow)
            .font_weight(FontWeight::SEMIBOLD)
    })
}

/// The status words either backend uses for a run that stopped for the user.
pub const ATTENTION_STATUSES: [&str; 3] = ["NeedsAttention", "AwaitingApproval", "WaitingForApproval"];

/// Whether a task belongs under My Attention: it is waiting on an answer, or it
/// has an approval question open right now. A run that merely failed is not
/// attention — the user cannot answer it.
pub fn needs_attention(status: &str, pending_approvals: usize) -> bool {
    ATTENTION_STATUSES.contains(&status) || pending_approvals > 0
}

/// Where an answered-or-not gated call stands, in shell words. `Expired` is the
/// honest third state: the run ended (or the service forgot the question) so the
/// answer can no longer land, which is not the same as the user not answering.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CardState {
    Pending,
    Answered,
    Expired,
}

/// One gated call the loop is stopped on, as a shell gathered it. Borrowed
/// fields because the card is built from the app's own record, per frame.
pub struct ApprovalCard<'a> {
    pub id: &'a str,
    /// What kind of call it is, as the tool table names it (`SHELL`, `EDIT`, …).
    pub category: &'a str,
    pub title: &'a str,
    /// `"high"` / `"medium"` / `"low"` / anything else, as the gate table said.
    pub risk: &'a str,
    pub reason: &'a str,
    pub path: Option<&'a str>,
    /// The call itself — the command line or the argument line.
    pub detail: &'a str,
    pub state: CardState,
    /// The raw decision key once answered.
    pub decision: &'a str,
    /// Why it is no longer answerable, for the expired row.
    pub note: &'a str,
}

/// The card for a gated call: what it is, why it is risky, the four answers the
/// loop accepts, and what happened to the answer. `on_answer` gets the raw
/// decision key, which is the only word the service understands.
pub fn approval_card<F>(t: &Theme, card: &ApprovalCard, on_answer: F) -> impl IntoElement
where
    F: Fn(&str, &mut Window, &mut App) + Clone + 'static,
{
    let live = card.state == CardState::Pending;
    let dot = risk_color(t, card.risk);
    let mut out = div().id(ElementId::Name(format!("appr-{}", card.id).into()))
        .rounded_md().border_1().border_color(t.border).bg(t.muted).px_3().py_2().flex().flex_col().gap_1();
    out = out
        .child(div().flex().items_center().justify_between()
            .child(div().flex().items_center().gap_2()
                .child(div().px_1().rounded_md().bg(dot)
                    .child(Label::new(card.category).font_family("monospace").text_size(px(10.0)).text_color(t.background)))
                .child(Label::new(card.title).text_size(px(12.5))))
            .child(Label::new(format!("{} risk", card.risk)).text_size(px(11.0)).text_color(dot)));
    if !card.reason.is_empty() {
        out = out.child(Label::new(card.reason).text_size(px(11.5)).text_color(t.muted_foreground));
    }
    if let Some(path) = card.path {
        out = out.child(Label::new(format!("file · {path}")).font_family("monospace").text_size(px(11.0)).text_color(t.muted_foreground));
    }
    out = out.child(Label::new(card.detail).font_family("monospace").text_size(px(11.0)).text_color(t.muted_foreground));
    let mut buttons = div().flex().flex_row().flex_wrap().gap_2().mt_1();
    for decision in APPROVAL_DECISIONS {
        let cb = on_answer.clone();
        buttons = buttons.child(
            Button::new(ElementId::Name(format!("appr-{}-{}", card.id, decision).into()))
                .label(decision_label(decision)).ghost().small()
                .when(!live, |b| b.disabled(true))
                .on_click(move |_, window, cx| cb(decision, window, cx)),
        );
    }
    out = out.child(buttons);
    out.child(match card.state {
        CardState::Pending => Label::new("the run waits on this answer").text_size(px(10.5)).text_color(t.yellow),
        CardState::Answered => Label::new(format!("answer sent · {}", decision_label(card.decision))).text_size(px(10.5)).text_color(t.success),
        CardState::Expired => Label::new(format!("not used · {}", card.note)).text_size(px(10.5)).text_color(t.red),
    })
}

/// The answered-command list. Rows are clickable and `on_select` gets the task
/// id, which is how a click jumps the rail to the task the call was made in.
pub fn approved_commands_panel<F>(t: &Theme, items: &[ApprovedCommand], on_select: F) -> Div
where
    F: Fn(String, &mut Window, &mut App) + Clone + 'static,
{
    let mut panel = div().flex().flex_col().gap_1()
        .child(Label::new("APPROVED COMMANDS")
            .text_size(px(10.0)).text_color(t.muted_foreground).font_weight(FontWeight::MEDIUM));

    for (i, item) in items.iter().enumerate() {
        let cb = on_select.clone();
        let task_id = item.task_id.clone();
        let dot = risk_color(t, &item.risk);
        let decision = decision_label(&item.decision);
        // A clickable div, not a Button: every button size in the toolkit fixes
        // its height, and this row is two lines tall, so the answer line got
        // sliced off. The command wraps instead of ellipsizing — the row exists
        // to say what was answered, and a cut-off path says nothing.
        panel = panel.child(
            div().id(ElementId::NamedInteger("appr-row".into(), i as u64))
                .w_full().flex().flex_col().gap_0p5().rounded_md().px_2().py(px(6.0))
                .cursor_pointer().hover(|this| this.bg(t.muted))
                .child(Label::new(item.command.clone()).text_size(px(11.5)).text_color(t.foreground))
                .child(div().flex().items_center().gap_1().overflow_hidden()
                    .child(div().w(px(5.0)).h(px(5.0)).flex_shrink_0().rounded_full().bg(dot))
                    .child(Label::new(if decision.is_empty() { item.risk.clone() } else { decision.to_string() })
                        .text_size(px(10.0)).text_color(t.muted_foreground))
                    .child(Label::new(format!("· {}", item.task_title))
                        .text_size(px(10.0)).text_color(t.accent).truncate()))
                .on_click(move |_, window, cx| cb(task_id.clone(), window, cx)),
        );
    }

    panel.children(items.is_empty().then(|| {
        Label::new("No answered commands yet.").text_size(px(11.0)).text_color(t.muted_foreground)
    }))
}

#[cfg(test)]
mod tests {
    // Named imports, not `use super::*`: this module's glob reaches the
    // `gpui_kit::test` attribute macro (re-exported from `gpui`), which
    // `#[test]` then resolves to instead of the built-in harness.
    use crate::attention::{
        decision_label, needs_attention, parse_risk, APPROVAL_DECISIONS, ATTENTION_STATUSES, Risk,
    };

    /// The service refuses anything outside these four, so the list never has to
    /// invent a word for a fifth answer — but an unknown key must not render as
    /// a made-up label either.
    #[test]
    fn every_decision_the_service_accepts_has_a_label() {
        for decision in APPROVAL_DECISIONS {
            assert!(!decision_label(decision).is_empty(), "{decision} has no label");
        }
        assert_eq!(APPROVAL_DECISIONS.len(), 4);
        assert_eq!(decision_label("allow_forever"), "");
    }

    #[test]
    fn risk_words_are_the_ones_the_gate_table_reports() {
        assert_eq!(parse_risk("high"), Risk::High);
        assert_eq!(parse_risk("medium"), Risk::Medium);
        assert_eq!(parse_risk("low"), Risk::Low);
        assert_eq!(parse_risk("unknown"), Risk::Unknown);
        // A word the table never uses is unknown, not a colour nobody chose.
        assert_eq!(parse_risk(""), Risk::Unknown);
        assert_eq!(parse_risk("catastrophic"), Risk::Unknown);
    }

    /// The two shells name a stopped run differently (`AwaitingApproval` for the
    /// core loop, `WaitingForApproval` for the task model), and either may carry
    /// an open question without having changed status yet.
    #[test]
    fn attention_is_an_open_question_not_a_failure() {
        for status in ATTENTION_STATUSES {
            assert!(needs_attention(status, 0), "{status} should ask for attention");
        }
        assert!(!needs_attention("Failed", 0));
        assert!(!needs_attention("Running", 0));
        assert!(needs_attention("Running", 1));
    }

    /// The shells read the same `/api/agent/poll` steps, so the words a gated
    /// call gets must come from one table — otherwise the two rails describe the
    /// same `run_shell` differently.
    #[test]
    fn a_call_is_described_from_the_loop_tool_table() {
        use crate::attention::{args_line, clip, describe_call, path_of, risk_word};
        let args = serde_json::json!({ "cmd": "cargo test", "timeout": 600 });
        assert_eq!(describe_call("run_shell", &args), ("CMD".into(), "Run command cargo test".into()));
        assert_eq!(path_of(&serde_json::json!({ "path": " src/a.py " })), Some("src/a.py".into()));
        assert_eq!(path_of(&serde_json::json!({ "path": "  " })), None);
        assert!(args_line(&args).contains("cargo test"));
        assert_eq!(risk_word(Some("High")), "high");
        assert_eq!(risk_word(Some("catastrophic")), "unknown");
        assert_eq!(risk_word(None), "unknown");
        assert_eq!(clip("  hi  ", 40), "hi");
        assert_eq!(clip(&"字".repeat(50), 3).chars().count(), 3);
    }

    /// Only the tools that put bytes on disk deliver a file, and the closing
    /// summary is built from the calls rather than the model's prose: two writes
    /// to one path are one delivered file.
    #[test]
    fn a_write_delivers_and_is_listed_once() {
        use crate::attention::{delivery_summary, delivery_verb, delivery_verb_from_title};
        assert_eq!(delivery_verb("write_file"), Some("created"));
        assert_eq!(delivery_verb("edit_file"), Some("updated"));
        // A read names a path but changes nothing, so it delivers nothing.
        assert_eq!(delivery_verb("read_file"), None);
        assert_eq!(delivery_verb("run_shell"), None);
        // The title spelling resolves to the same verbs the name spelling does.
        assert_eq!(delivery_verb_from_title("Write file src/a.py"), Some("created"));
        assert_eq!(delivery_verb_from_title("Edit file notes/todo list.md"), Some("updated"));
        assert_eq!(delivery_verb_from_title("Read file src/a.py"), None);
        assert_eq!(delivery_verb_from_title(""), None);
        assert_eq!(delivery_summary(&[]), "");
        let items = [("src/a.py", "created"), ("src/a.py", "updated"), (".local-docs/b.md", "updated")];
        assert_eq!(
            delivery_summary(&items),
            "**Delivered 2 files**\n\n- `src/a.py` — created\n- `.local-docs/b.md` — updated"
        );
        assert!(delivery_summary(&[("src/a.py", "created")]).contains("1 file**"));
    }
}
