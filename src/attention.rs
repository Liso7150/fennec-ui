//! Shared "My Attention" rail: the badge that counts what still waits for the
//! user, and the list of commands the user has already answered.
//!
//! Every gated tool call goes through the same four-answer question, so the
//! answer words, the risk colors, and the row layout live here once instead of
//! drifting between the native shell and the PWA that show them. The app still
//! owns the gathering: it hands over the answered approvals its own backend
//! reported.

use gpui_kit::component::{
    Disableable,
    button::{Button, ButtonVariants},
    label::Label,
    Sizable, Theme,
};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::Value;

/// `fennec-core/loop.py:_APPROVAL_OPTIONS`. The answers a shell may send, and
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

/// The tool names the loop advertises (`fennec-core/tools.py:TOOLS`). An
/// object that names one of these is the model spelling a call; an object
/// naming anything else is data the user asked for.
pub const TOOL_NAMES: [&str; 14] = [
    "list_dir", "read_file", "write_file", "edit_file", "grep", "semantic_search",
    "run_python", "run_shell", "git_status", "git_diff", "web_task", "coworker_task",
    "propose_queue", "summarise",
];

/// The fence languages that mean "run this in a shell" — the same set the loop
/// treats as a call it could have made itself (`loop.py:_SHELL_LANGS`).
pub const SHELL_LANGS: [&str; 9] =
    ["bash", "sh", "shell", "zsh", "ksh", "powershell", "pwsh", "cmd", "bat"];

pub fn is_tool_name(s: &str) -> bool {
    TOOL_NAMES.iter().any(|n| *n == s)
}

/// The language and body of every ``` fence, and the text left outside them.
fn fences(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        let after = &rest[open + 3..];
        let Some(nl) = after.find('\n') else { break };
        let (head, tail) = (&after[..nl], &after[nl + 1..]);
        let (body, used) = match tail.find("```") {
            Some(i) => (&tail[..i], i + 3),
            None => (tail, tail.len()),
        };
        out.push((head.trim().to_lowercase(), body.to_string()));
        rest = &tail[used..];
    }
    out
}

fn outside_fences(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 3..];
        let Some(nl) = after.find('\n') else { break };
        let tail = &after[nl + 1..];
        rest = match tail.find("```") {
            Some(i) => &tail[i + 3..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

/// Whether a JSON value is, or holds, an object naming an advertised tool.
fn value_names_tool(v: &Value) -> bool {
    match v {
        Value::Object(o) => ["name", "tool", "type", "command"]
            .iter()
            .filter_map(|k| o.get(*k).and_then(|x| x.as_str()))
            .any(is_tool_name)
            || o.values().any(value_names_tool),
        Value::Array(a) => a.iter().any(value_names_tool),
        _ => false,
    }
}

/// The same test for a call the model ran out of room writing, which no parser
/// can read: read the first string that follows a name slot.
fn head_names_tool(text: &str) -> bool {
    for key in ["\"name\"", "\"tool\"", "\"type\"", "\"command\""] {
        let mut from = 0;
        while let Some(rel) = text[from..].find(key) {
            let after = &text[from + rel + key.len()..];
            let Some(colon) = after.find(':') else { break };
            let rest = &after[colon + 1..];
            let Some(open) = rest.find('"') else { break };
            let body = &rest[open + 1..];
            let Some(close) = body.find('"') else { break };
            if is_tool_name(&body[..close]) {
                return true;
            }
            from += rel + key.len();
        }
    }
    false
}

/// Whether an object/array body is a spelled call.
fn json_is_call(body: &str) -> bool {
    let t = body.trim_start();
    if !t.starts_with('{') && !t.starts_with('[') {
        return false;
    }
    serde_json::from_str::<Value>(t).map(|v| value_names_tool(&v)).unwrap_or(false)
        || head_names_tool(t)
}

/// Text that is a tool call the model spelled out rather than an answer: the
/// bare object, the `<tool_code>` tag, the same shapes inside a code fence, a
/// fence named after a tool, or a reply that is nothing but shell blocks.
///
/// Read from the text alone, so it also works on restored history, where the
/// step's call list is gone. A fence holding ordinary code, and any prose
/// around it, is formatted output and stays on screen.
pub fn call_blob(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if t.starts_with('<') {
        return true;
    }
    if t.starts_with('{') || t.starts_with('[') {
        return json_is_call(t);
    }
    let blocks = fences(t);
    if blocks.is_empty() {
        return t.contains("<tool_code>");
    }
    for (lang, body) in &blocks {
        let name = lang.strip_prefix("tool:").unwrap_or(lang).trim();
        if is_tool_name(name) {
            return true;
        }
        let b = body.trim();
        if json_is_call(b) {
            return true;
        }
        if "<tool_code>" == b || b.starts_with("<tool_code>") {
            return true;
        }
        // `read_file path=…` spelled on the first line of a plain fence.
        if let Some(line) = b.lines().next() {
            let head = line.trim().strip_prefix("tool_").unwrap_or_else(|| line.trim());
            let head = head.split([' ', '\t', '=', '(']).next().unwrap_or("");
            if is_tool_name(head) {
                return true;
            }
        }
    }
    // Nothing but command blocks: the loop calls that a call it failed to make
    // (`loop.py:_only_command_fences`), so the shell calls it a blob too.
    outside_fences(t).trim().is_empty()
        && blocks.iter().any(|(l, b)| SHELL_LANGS.iter().any(|s| *s == l.as_str()) && !b.trim().is_empty())
}

/// Text that is the model spelling a tool call rather than answering, so a
/// shell can keep it out of the answer area: every turn that fails to land a
/// call spells the same blob again, and the call's own row already says what
/// ran.
///
/// A shape the text carries on its own is a call whatever the step holds. A
/// fence beside prose counts when the loop read a call out of that same step,
/// and a fence that is itself a call object counts without that help: history
/// restored from disk has no call list to consult.
pub fn looks_like_call(text: &str, made_call: bool) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if call_blob(t) {
        return true;
    }
    match t.chars().next() {
        Some('{') | Some('[') | Some('<') => true,
        Some('`') => t.starts_with("```") && made_call,
        _ => made_call && ["\"tool\":", "\"name\":", "\"command\":", "\"args\":", "\"arguments\":"]
            .iter()
            .any(|key| t.contains(key)),
    }
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

/// The words a gated call gets on a card and in the answered list, whichever
/// half of the record a shell happens to hold. The loop names the tool
/// (`run_shell`) and describes the call (`Run command ls`); either spelling has
/// to resolve to the same row, because a raw tool name is not something the
/// user can read as a question they answered.
///
/// A described title is already in the user's words and is kept whole; a bare
/// tool name is run back through `describe_call` with no arguments, which gives
/// the human phrase without a target.
pub fn gate_title(name: &str, described: Option<&str>) -> String {
    if let Some(t) = described.map(str::trim).filter(|t| !t.is_empty()) {
        if t != name {
            return t.to_string();
        }
    }
    let (_, title) = describe_call(name, &Value::Null);
    title.trim().to_string()
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
/// spellings have to resolve to one verb or the native shell and the PWA would
/// describe the same run differently.
pub fn delivery_verb_from_title(title: &str) -> Option<&'static str> {
    match title.split_whitespace().next()?.to_ascii_lowercase().as_str() {
        "write" => Some("created"),
        "edit" => Some("updated"),
        _ => None,
    }
}

/// What a write's display title says, in the two things a file row needs: the
/// verb the call used and the path it named. Shells that kept only the title (the
/// coding agent stores no arguments on the message) still have to answer "which
/// file, and did it create or update it", and they must not count the same write
/// twice because the loop also delivered a chip for it.
///
/// A leading category badge is stripped first: the coding agent spells a call
/// `"EDIT · Write file src/a.py"`, and only the `EDIT` badge is ever a write, so
/// nothing else is split on the separator (a path may contain ` · ` itself).
pub fn write_of_title(title: &str) -> Option<(&'static str, String)> {
    let title = match title.split_once(" · ") {
        Some(("EDIT", rest)) => rest,
        _ => title,
    };
    let verb = delivery_verb_from_title(title)?;
    // `describe_call` writes "Write file <path>" and "Edit file <path>"; the
    // path is everything after the word `file`, because a path may contain
    // spaces and must survive intact.
    let path = title.split_once(" file ")?.1.trim();
    (!path.is_empty()).then(|| (verb, path.to_string()))
}

/// The delivered-file list that used to be written into the last answer as
/// prose. A file is delivered as a chip now, and the chip carries the same
/// words, so the paragraph is trimmed out of history rather than shown twice.
/// Returns `true` when that leaves the message with nothing else in it.
pub fn strip_delivery_prose(text: &mut String) -> bool {
    let Some(at) = text.find("**Delivered ") else { return false };
    text.truncate(at);
    let kept = text.trim_end().to_string();
    *text = kept;
    text.is_empty()
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

/// Whether a gated call still belongs in the timeline. An answer the user has
/// already sent is not a question, so the card leaves the run and the call goes
/// to My Attention, where the answered list keeps it. An expired one stays: it
/// says why a run stopped with nothing to click, which the user has to be told.
/// The PWA mirrors this on `meta.status`.
pub fn card_shows(state: CardState) -> bool {
    state != CardState::Answered
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
    // The call line is the machine's view of the gate (the arguments, compact).
    // A shell whose title already spells the call out in words has nothing left
    // to add here, and a second identical line reads as a bug, not as detail.
    if !card.detail.is_empty() && card.detail != card.title {
        out = out.child(Label::new(card.detail).font_family("monospace").text_size(px(11.0)).text_color(t.muted_foreground));
    }
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
        call_blob, decision_label, looks_like_call, needs_attention, parse_risk,
        APPROVAL_DECISIONS, ATTENTION_STATUSES, Risk,
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

    /// The two backends name a stopped run differently (`AwaitingApproval` for the
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

    /// Every surface reads the same `/api/agent/poll` steps, so the words a gated
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

    /// Each backend names a gated call from whichever half of the record it
    /// kept, and neither spelling may put a raw tool name in front of the user:
    /// `run_shell` is the loop's word for the call, not what the call does.
    #[test]
    fn a_gated_call_is_never_labelled_with_its_tool_name() {
        use crate::attention::{describe_call, gate_title};
        let described = "Run command cargo test";
        assert_eq!(gate_title("run_shell", Some(described)), described);
        // A record that only kept the tool name still reads as a command.
        assert_eq!(gate_title("run_shell", None), "Run command");
        assert_eq!(gate_title("read_file", Some("  ")), "Read file");
        // A record whose title is just the tool name repeated is the same case.
        assert_eq!(gate_title("run_shell", Some("run_shell")), "Run command");
        // An unknown tool has no human phrase, so its name is all there is.
        assert_eq!(gate_title("warp_drive", None), "warp_drive");
        // The two spellings agree with the table the timeline rows use.
        let bare = describe_call("run_python", &serde_json::json!({})).1;
        assert_eq!(gate_title("run_python", None), bare.trim());
    }

    /// An answered card is removed from the run; a pending one and an expired one
    /// are not, because each still tells the user something they cannot learn
    /// from the answer alone — what it waits on, or why nothing waits any more.
    #[test]
    fn an_answer_leaves_the_timeline_and_a_dead_question_stays() {
        use crate::attention::{CardState, card_shows};
        assert!(card_shows(CardState::Pending));
        assert!(card_shows(CardState::Expired));
        assert!(!card_shows(CardState::Answered));
    }

    /// Only the tools that put bytes on disk deliver a file, and the chip — not
    /// a paragraph — is what says so: two writes to one path are one delivered
    /// file, and the prose list older history holds is trimmed out of the answer.
    #[test]
    fn a_write_delivers_and_is_listed_once() {
        use crate::attention::{delivery_verb, delivery_verb_from_title, strip_delivery_prose, write_of_title};
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
        // The agent's badge-prefixed spelling resolves to the same pair, and a
        // path with spaces or the separator itself survives.
        assert_eq!(write_of_title("EDIT · Write file src/a.py"), Some(("created", "src/a.py".into())));
        assert_eq!(write_of_title("Edit file notes/todo list.md"), Some(("updated", "notes/todo list.md".into())));
        assert_eq!(write_of_title("EDIT · Write file a · b.md"), Some(("created", "a · b.md".into())));
        assert_eq!(write_of_title("CMD · Run command ls · list"), None);
        assert_eq!(write_of_title("Write file "), None);
        // An answer that also wrote something keeps its prose; the file list goes.
        let mut kept = "Made the change.\n\n**Delivered 2 files**\n\n- `src/a.py` — created".to_string();
        assert!(!strip_delivery_prose(&mut kept));
        assert_eq!(kept, "Made the change.");
        // An answer that was nothing but the list is empty afterwards, so the
        // caller drops the message instead of drawing an empty bubble.
        let mut only = "**Delivered 1 file**\n\n- `src/a.py` — created".to_string();
        assert!(strip_delivery_prose(&mut only));
        assert_eq!(only, "");
        // Text the model wrote that merely mentions the word stays untouched.
        let mut plain = "I delivered the file you asked for.".to_string();
        assert!(!strip_delivery_prose(&mut plain));
        assert_eq!(plain, "I delivered the file you asked for.");
    }

    /// A call blob never belongs in the answer area; a formatted code answer
    /// always does.
    #[test]
    fn a_call_spelling_is_not_an_answer() {
        // The shape the loop publishes beside the call it is about to run.
        let blob = r#"{"name": "read_file", "args": {"path": "checklist.md"}}"#;
        assert!(looks_like_call(blob, true));
        // And when no call landed: that turn failed to parse, and the next one
        // spells the same object again, which is the repetition to get rid of.
        assert!(looks_like_call(blob, false));
        assert!(looks_like_call("\u{3c}function=run_shell\u{3e}", false));
        // A fence is a call only when the loop read one out of that step. The
        // same fence alone is the model answering with formatted code.
        let fence = "```rust\nfn main() {}\n```";
        assert!(!looks_like_call(fence, false));
        assert!(looks_like_call(fence, true));
        // Prose answers survive either way, including one naming a parameter.
        assert!(!looks_like_call("Made the change in `checklist.md`.", true));
        assert!(!looks_like_call("\"args\": the bag", false));
        assert!(!looks_like_call("", true));
        assert!(!looks_like_call("   ", false));
    }

    /// The shapes restored history actually holds, taken verbatim from a real
    /// `history.json`: none of them carry a call list, so the text alone has
    /// to say they are not answers.
    #[test]
    fn a_fenced_call_spelling_is_not_an_answer_either() {
        let fenced = "```json\n{\"name\": \"read_file\", \"args\": {\"path\": \"notes.txt\"}}\n```";
        assert!(call_blob(fenced));
        assert!(looks_like_call(fenced, false));
        // A queue proposal is an array of the same envelope.
        let arr = "```json\n[\n  {\n    \"type\": \"run_shell\",\n    \"command\": \"dir /s /b\"\n  }\n]\n```";
        assert!(call_blob(arr));
        // The fence named after the tool, and the tool named on its first line.
        assert!(call_blob("```read_file\npath=.local-docs/queue-probe/notes.txt\n```"));
        assert!(call_blob("```text\nread_file path=notes.txt\n```"));
        assert!(call_blob("```text\nwrite_file path=checklist7.md content=\"1. a\"\n```"));
        // A reply that is nothing but a command the loop could have run.
        assert!(call_blob("```bash\ncat .local-docs/queue-probe/notes.txt\n```"));
        // Genuine answers keep their formatting.
        assert!(!call_blob("```python\nprint(sum(range(10)))\n```"));
        assert!(!call_blob("```json\n{\"enabled\": true, \"name\": \"fennec\"}\n```"));
        assert!(!call_blob("Use ```bash\nls -R\n``` to list the tree, then check the count."));
        assert!(!call_blob("The checklist has 3 lines."));
        assert!(!call_blob(""));
    }
}
