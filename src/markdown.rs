//! The model's text, laid out — shared markdown subset rendering.
//!
//! Element ids are offset by `msg_idx` so several messages on one screen never
//! fold into duplicate a11y node ids (a debug-build panic when a UIA client
//! is connected).

use gpui_kit::component::{label::Label, scroll::ScrollableElement, Theme};
use gpui_kit::prelude::*;
use gpui_kit::*;

/// One block between code fences: prose the Markdown subset covers, or a fence
/// the shell shows as a monospace card.
pub enum Block {
    Prose(String),
    Code { lang: String, body: String },
}

/// Split ``` fences out of a message: prose outside the fences, and a fence's
/// first line is a language only when it looks like one.
pub fn rich_blocks(text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        let before = &rest[..open];
        if !before.trim().is_empty() {
            out.push(Block::Prose(before.trim().to_string()));
        }
        let after = &rest[open + 3..];
        let (body, tail) = match after.find("```") {
            Some(end) => (&after[..end], &after[end + 3..]),
            None => (after, ""),
        };
        let body = body.trim();
        let (lang, code) = match body.split_once('\n') {
            Some((first, code)) => match info_string(first) {
                Some(lang) => (lang.to_string(), code.trim().to_string()),
                None => (String::new(), body.to_string()),
            },
            None => (String::new(), body.to_string()),
        };
        out.push(Block::Code { lang, body: code });
        rest = tail;
    }
    if !rest.trim().is_empty() {
        out.push(Block::Prose(rest.trim().to_string()));
    }
    out
}

/// A fence's first line, when it names a language instead of holding code.
fn info_string(line: &str) -> Option<&str> {
    let t = line.trim();
    (!t.is_empty()
        && t.len() <= 16
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_' | '.')))
        .then_some(t)
}

/// One styled piece of a line.
#[derive(Clone, PartialEq, Debug)]
pub enum Run {
    Text(String),
    Code(String),
    Strong(String),
    Em(String),
    Link { label: String, url: String },
}

#[derive(Clone, PartialEq, Debug)]
pub struct MdLine {
    pub head: u8,
    pub bullet: bool,
    pub runs: Vec<Run>,
}

/// The Markdown subset these models actually emit: `#` headings, `-` bullets,
/// `code`, **strong**, *emphasis*, [label](url). Numbered items keep the
/// model's own digits, because the number is part of the answer.
pub fn md_lines(text: &str) -> Vec<MdLine> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        let (head, body) = if hashes > 0 && hashes <= 4 && line[hashes..].starts_with(' ') {
            (hashes as u8, line[hashes..].trim_start())
        } else {
            (0u8, line)
        };
        let mut bullet = false;
        let mut rest = body;
        for m in ["- ", "* ", "+ "] {
            if let Some(r) = rest.strip_prefix(m) {
                bullet = true;
                rest = r;
                break;
            }
        }
        out.push(MdLine { head, bullet, runs: md_runs(rest) });
    }
    out
}

/// Inline parsing. A marker that never closes stays literal text, so a stray
/// asterisk or bracket can never swallow the rest of the sentence.
fn md_runs(text: &str) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut buf = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let mut best: Option<(usize, usize, u8)> = None;
        for (idx, len, kind) in [
            (rest.find('`'), 1usize, 1u8),
            (rest.find("**"), 2, 2),
            (rest.find('*'), 1, 3),
            (rest.find('['), 1, 4),
        ] {
            let Some(i) = idx else { continue };
            let wins = match best {
                None => true,
                Some((bi, bl, _)) => i < bi || (i == bi && len > bl),
            };
            if wins {
                best = Some((i, len, kind));
            }
        }
        let Some((at, open_len, kind)) = best else {
            buf.push_str(rest);
            break;
        };
        buf.push_str(&rest[..at]);
        rest = &rest[at..];
        if kind == 4 {
            match md_link(rest) {
                Some((label, url, used)) => {
                    if !buf.is_empty() {
                        runs.push(Run::Text(std::mem::take(&mut buf)));
                    }
                    runs.push(Run::Link { label, url });
                    rest = &rest[used..];
                }
                None => {
                    buf.push('[');
                    rest = &rest[1..];
                }
            }
            continue;
        }
        let close_len = open_len;
        let body = &rest[open_len..];
        match body.find(&rest[..close_len]) {
            Some(end) if end > 0 => {
                if !buf.is_empty() {
                    runs.push(Run::Text(std::mem::take(&mut buf)));
                }
                let text = body[..end].to_string();
                runs.push(match kind {
                    1 => Run::Code(text),
                    2 => Run::Strong(text),
                    _ => Run::Em(text),
                });
                rest = &rest[open_len + end + close_len..];
            }
            _ => {
                buf.push_str(&rest[..open_len]);
                rest = &rest[open_len..];
            }
        }
    }
    if !buf.is_empty() {
        runs.push(Run::Text(buf));
    }
    runs
}

/// `[label](url)` at the front of `s`, plus how many bytes it used.
fn md_link(s: &str) -> Option<(String, String, usize)> {
    let after = s.strip_prefix('[')?;
    let split = after.find("](")?;
    let label = &after[..split];
    let tail = &after[split + 2..];
    let end = tail.find(')')?;
    Some((label.to_string(), tail[..end].to_string(), 1 + split + 2 + end + 1))
}

/// Chat text as the model wrote it. ``` fences become monospace cards; the
/// prose between them is read as a Markdown subset. `msg_idx` must be unique
/// per message on screen so the element ids never collide.
pub fn rich_div(t: &Theme, text: &str, dim: bool, msg_idx: usize) -> Div {
    let base = if dim { 12.0 } else { 13.5 };
    let mut col = div().flex().flex_col().gap_2().max_w(px(680.0));
    for (bi, b) in rich_blocks(text).into_iter().enumerate() {
        let block_id = msg_idx * 1000 + bi;
        col = col.child(div().id(ElementId::NamedInteger("blk".into(), block_id as u64)).child(match b {
            Block::Prose(p) => md_div(t, &p, base, dim, msg_idx),
            Block::Code { lang, body } => code_block(t, &lang, &body),
        }));
    }
    col
}

/// Prose under the Markdown subset: headings size up, bullets get a marker,
/// inline runs keep their own styles.
pub fn md_div(t: &Theme, text: &str, base: f32, dim: bool, msg_idx: usize) -> Div {
    let mut col = div().flex().flex_col().gap_1();
    for (li, line) in md_lines(text).into_iter().enumerate() {
        if line.runs.is_empty() {
            continue;
        }
        let size = base + match line.head { 1 => 6.0, 2 => 3.5, 3 => 1.5, _ => 0.0 };
        let body = if line.head > 0 { t.foreground } else if dim { t.muted_foreground } else { t.foreground };
        let line_id = msg_idx * 10000 + li;
        let mut row = div().id(ElementId::NamedInteger("mdl".into(), line_id as u64)).flex().flex_row().flex_wrap().items_center();
        if line.bullet {
            row = row.child(div().flex_shrink_0().child(Label::new("•").text_size(px(size)).text_color(body)));
        }
        for (ri, run) in line.runs.iter().enumerate() {
            let run_id = msg_idx * 100000 + li * 100 + ri;
            let el = run_div(t, run, size, body, line.head > 0, run_id);
            // A run without a space is one token — the path of a delivered file,
            // a command — and in this row gpui sizes it to its own width, so it
            // runs past the pane and the rest of it is never drawn. Handing it
            // the row's leftover width as a box makes the text element break it
            // mid-token, the way the group rows and the approved-commands list do.
            let token = match run {
                Run::Text(s) | Run::Strong(s) | Run::Em(s) | Run::Code(s) => !s.contains(' '),
                Run::Link { .. } => false,
            };
            row = row.child(if token {
                div().flex().flex_col().flex_1().overflow_hidden().child(el)
            } else {
                div().child(el)
            });
        }
        col = col.child(row);
    }
    col
}

pub fn run_div(t: &Theme, run: &Run, size: f32, body: Hsla, heading: bool, idx: usize) -> impl IntoElement {
    let id = ElementId::NamedInteger("mdr".into(), idx as u64);
    match run {
        Run::Text(s) => {
            let mut l = Label::new(s.clone()).text_size(px(size)).text_color(body);
            if heading {
                l = l.font_weight(FontWeight::SEMIBOLD);
            }
            div().id(id).child(l)
        }
        Run::Strong(s) => div().id(id).child(
            Label::new(s.clone()).text_size(px(size)).text_color(t.foreground).font_weight(FontWeight::SEMIBOLD)
        ),
        Run::Em(s) => div().id(id).italic().child(Label::new(s.clone()).text_size(px(size)).text_color(body)),
        Run::Code(s) => div().id(id).rounded_md().bg(t.muted).px_1().font_family("monospace").child(
            Label::new(s.clone()).text_size(px((size - 1.0).max(9.0))).text_color(t.foreground)
        ),
        Run::Link { label, url } => div().id(id).flex().flex_row().flex_wrap().items_center()
            .child(Label::new(label.clone()).text_size(px(size)).text_color(t.primary))
            .child(Label::new(format!(" ({url})")).font_family("monospace")
                .text_size(px((size - 1.5).max(9.0))).text_color(t.muted_foreground)),
    }
}

pub fn code_block(t: &Theme, lang: &str, code: &str) -> Div {
    let mut col = div().flex().flex_col().gap_1().max_w(px(680.0))
        .rounded_md().border_1().border_color(t.border).bg(t.muted).px_3().py_2();
    if !lang.is_empty() {
        col = col.child(div().font_family("monospace")
            .child(Label::new(lang.to_string()).text_size(px(10.0)).text_color(t.muted_foreground)));
    }
    col = col.child(div().font_family("monospace").max_h(px(240.0)).overflow_y_scrollbar()
        .child(Label::new(code.to_string()).text_size(px(12.0)).text_color(t.foreground)));
    col
}

/// What a produced file is called in the artifact card's subtitle.
pub fn artifact_kind(name: &str) -> &'static str {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "md" => "Markdown attachment",
        "rs" => "Rust source",
        "py" => "Python source",
        "json" | "toml" | "yaml" | "yml" => "Config file",
        "txt" => "Text file",
        "js" | "ts" | "tsx" | "jsx" => "Script",
        "html" => "HTML",
        "htm" => "HTML",
        "css" => "CSS",
        _ => "File attachment",
    }
}

/// The extension shown inside the delivered file's badge tile, uppercased and
/// capped at four characters so an unusual suffix still fits the square.
pub fn artifact_badge(name: &str) -> String {
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("file");
    let upper = ext.to_ascii_uppercase();
    if upper.chars().count() <= 4 {
        upper
    } else {
        upper.chars().take(4).collect()
    }
}

/// The delivered file's second line: "<dir> · <kind>". A file at the workspace
/// root has no directory to name, so it shows the kind alone.
pub fn attachment_subtitle(path: &str, name: &str) -> String {
    let kind = artifact_kind(name);
    let dir = std::path::Path::new(path).parent()
        .map(|d| d.to_string_lossy().to_string())
        .filter(|d| !d.is_empty() && d != "." && d != "./")
        .unwrap_or_default();
    if dir.is_empty() { kind.to_string() } else { format!("{dir} · {kind}") }
}

/// Diff-shaped line counts (+/−, fence headers excluded) over whatever text a
/// tool card carries; (0, 0) means the payload held no diff to count.
pub fn diff_counts(text: &str) -> (usize, usize) {
    let (mut adds, mut dels) = (0usize, 0usize);
    for line in text.lines() {
        if line.starts_with('+') && !line.starts_with("+++") { adds += 1; }
        else if line.starts_with('-') && !line.starts_with("---") { dels += 1; }
    }
    (adds, dels)
}