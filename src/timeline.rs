//! Shared grouping of a run's tool calls into the blocks every Fennec surface
//! shows: one expandable block per stretch between two answers, holding every
//! action of that stretch — the files it wrote and the tools it ran together —
//! and nothing else. A call the run is still inside stays on its own so its
//! output can be watched live.
//!
//! Only the newest block is open by default: it is the one the run is still in,
//! and the answer that follows it closes it.
//!
//! The shells own what a call *is* (the desktop keeps a `ToolEvent` per call,
//! the coding agent a message per step, the PWA a JSON message); what lives here
//! is the decision of where it goes in the timeline and the words on the block,
//! so the three surfaces cannot describe one run differently.

/// A call as the grouping sees it: did it write a file, and has it returned.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CallShape {
    /// The call put a file on disk (a write/edit, or its delivered chip).
    pub wrote: bool,
    /// The call is still running, so nothing about it is settled yet.
    pub running: bool,
}

/// One stretch of calls: everything the run did between two answers, as indexes
/// back into the caller's own list in the order the calls were made.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Segment {
    /// How many turns the timeline had shown when these calls started. Calls
    /// that saw the same turns sit between the same two answers.
    pub after: usize,
    /// The settled calls — the rows of the block.
    pub settled: Vec<usize>,
    /// The calls in this stretch the run is still inside. Never folded into the
    /// block: their output is still arriving.
    pub live: Vec<usize>,
}

/// Cut the calls into one block per stretch between answers.
///
/// `after` is one entry per call — the turns shown when it started, in call
/// order — so a call that arrives after an answer opens the next block instead
/// of joining the one before it. A running call is kept out of the block but
/// stays in the stretch it belongs to.
pub fn group_segments(after: &[usize], shapes: &[CallShape]) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    for (i, shape) in shapes.iter().enumerate() {
        let key = after.get(i).copied().unwrap_or(0);
        if segments.last().map(|s| s.after) != Some(key) {
            segments.push(Segment { after: key, ..Default::default() });
        }
        let segment = segments.last_mut().unwrap();
        if shape.running {
            segment.live.push(i);
        } else {
            segment.settled.push(i);
        }
    }
    segments
}

/// Whether a block is drawn open. The click the user made wins in both
/// directions; otherwise only the newest block is open, because that is the one
/// the run is still inside. Once a new answer lands the next block becomes the
/// newest and this one closes by itself.
pub fn is_open(held_open: bool, held_closed: bool, newest: bool) -> bool {
    if held_open {
        true
    } else if held_closed {
        false
    } else {
        newest
    }
}

/// The count line on a block. It counts the rows the block holds, so a file the
/// run rewrote five times is still one line of work.
pub fn actions_title(n: usize) -> String {
    format!("{n} action{}", if n == 1 { "" } else { "s" })
}

/// Which of a run's write calls get a row: one per file, the last write of it
/// winning. A model that rewrote the same file six times delivered one file, and
/// a block that counted six would be counting its own repetition.
///
/// `paths` is one entry per call, in call order; an empty entry is a call that
/// named no file, and those never collapse into each other. The returned indexes
/// keep call order.
pub fn unique_writes(paths: &[&str]) -> Vec<usize> {
    let wrote: Vec<CallShape> = paths.iter().map(|_| CallShape { wrote: true, running: false }).collect();
    unique_actions(&wrote, paths)
}

/// Which of a block's calls get a row, in call order: every call that did not
/// write a file, and for each file the last write of it. A block of mixed
/// actions still says what the run delivered, not how often it repeated itself.
///
/// `paths` is one entry per call, in call order; an empty entry names no file
/// and so never collapses into another call.
pub fn unique_actions(shapes: &[CallShape], paths: &[&str]) -> Vec<usize> {
    let path = |i: usize| paths.get(i).copied().unwrap_or("");
    let mut last_of: Vec<(&str, usize)> = Vec::new();
    for (i, shape) in shapes.iter().enumerate() {
        if !shape.wrote || path(i).is_empty() {
            continue;
        }
        match last_of.iter().position(|(p, _)| *p == path(i)) {
            Some(at) => last_of[at].1 = i,
            None => last_of.push((path(i), i)),
        }
    }
    (0..shapes.len())
        .filter(|&i| {
            let wrote = shapes.get(i).map(|s| s.wrote).unwrap_or(false);
            !wrote || path(i).is_empty() || last_of.iter().any(|(_, last)| *last == i)
        })
        .collect()
}

/// One line inside a block: what the call was, and what came of it. Pure data —
/// the shells keep their own click handlers, because only they know whether a
/// row opens a file, reviews a diff, or has nothing behind it.
pub struct GroupRow {
    /// The file's own badge for a write, the category icon for a call.
    pub icon: String,
    /// The file's name for a call that names one, otherwise the command's first
    /// words — what `short_text` produced. The whole path or command stays with
    /// the call and is what its surface shows when the user opens the row.
    pub label: String,
    /// Lines added and removed, when the row changed a file.
    pub adds: usize,
    pub dels: usize,
    /// `created` / `updated` for a write, `Complete · 0.8s` for a call.
    pub status: String,
    /// A call that failed is the one row in a block the user must not miss.
    pub failed: bool,
}

impl GroupRow {
    pub fn new(icon: String, label: String, status: String) -> Self {
        Self { icon, label, adds: 0, dels: 0, status, failed: false }
    }

    /// A row for one settled call: `path` is the file it named — a read names one
    /// too, so this is not only for writes — and `title` is how the surface spells
    /// the call. The row shows the shortened words; the call keeps its whole text
    /// for when the user opens the row.
    pub fn call(icon: String, path: &str, title: &str, status: String) -> Self {
        Self::new(icon, short_text(path, title), status)
    }

    pub fn change(mut self, adds: usize, dels: usize) -> Self {
        self.adds = adds;
        self.dels = dels;
        self
    }

    pub fn failed(mut self) -> Self {
        self.failed = true;
        self
    }
}

/// How many words a shortened row keeps of a command. Enough to recognise the
/// call; the rest of it belongs to the expanded row, not to the block.
pub const ROW_WORDS: usize = 7;

/// A path's last element, accepting either separator so a Windows and a POSIX
/// path shorten the same way.
pub fn file_name(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches(['/', '\\']);
    let name = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    if name.is_empty() { trimmed.to_string() } else { name.to_string() }
}

/// The call's own words without the category badge some surfaces put in front of
/// them (`EDIT · Write file x.py`), because that badge is already the row icon.
fn strip_badge(title: &str) -> &str {
    match title.split_once(" · ") {
        Some((badge, rest))
            if badge.len() <= 6 && badge.chars().all(|c| c.is_ascii_uppercase() || c == '_') => rest,
        _ => title,
    }
}

/// The file a call's title names, when it names one: `describe_call` spells the
/// file calls `Read file <path>` / `Write file <path>` / `Edit file <path>`, and
/// the path is everything after the word `file` because it may hold spaces.
pub fn called_file(title: &str) -> Option<String> {
    let body = strip_badge(title);
    let lower = body.to_ascii_lowercase();
    ["read file ", "write file ", "edit file ", "delete file "]
        .iter()
        .find_map(|verb| lower.strip_prefix(verb))
        .map(|tail| &body[body.len() - tail.len()..])
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

/// The words a folded row shows: a call that names a file shows that file's name,
/// a command shows its first [`ROW_WORDS`] words and an ellipsis for the rest, so
/// a short row always says that it is short.
pub fn short_text(path: &str, title: &str) -> String {
    let path = path.trim();
    let file = (!path.is_empty())
        .then(|| path.to_string())
        .or_else(|| called_file(title));
    if let Some(file) = file {
        return file_name(&file);
    }
    let words: Vec<&str> = strip_badge(title).split_whitespace().collect();
    if words.len() <= ROW_WORDS {
        return words.join(" ");
    }
    format!("{} …", words[..ROW_WORDS].join(" "))
}

/// The text an open row or card shows: the whole path or command, then whatever
/// the call returned. Nothing is prefixed when the short label already was the
/// whole thing, so an unshortened call never prints itself twice.
pub fn reveal_text(short: &str, full: &str, body: &str) -> String {
    if short == full {
        return body.to_string();
    }
    if body.trim().is_empty() {
        return full.to_string();
    }
    format!("{full}\n\n{body}")
}

#[cfg(test)]
mod tests {
    use crate::timeline::{
        CallShape, GroupRow, Segment, actions_title, called_file, file_name, group_segments,
        is_open, reveal_text, short_text, unique_actions, unique_writes,
    };

    fn call(wrote: bool, running: bool) -> CallShape {
        CallShape { wrote, running }
    }

    /// The rule the three surfaces share: calls made between the same two
    /// answers share one block, whatever kind of call they were.
    #[test]
    fn the_actions_between_two_answers_are_one_block() {
        // Two reads and a write before the first answer, a write and a command
        // after it.
        let after = [1, 1, 1, 2, 2];
        let shapes = [
            call(false, false),
            call(false, false),
            call(true, false),
            call(true, false),
            call(false, false),
        ];
        assert_eq!(
            group_segments(&after, &shapes),
            vec![
                Segment { after: 1, settled: vec![0, 1, 2], live: vec![] },
                Segment { after: 2, settled: vec![3, 4], live: vec![] },
            ]
        );
    }

    /// Nothing that is still happening belongs in a block, because the count
    /// would be a promise — but it still belongs to the stretch it is in.
    #[test]
    fn a_running_call_stays_out_of_its_block() {
        let segments = group_segments(
            &[1, 1, 1, 1],
            &[call(true, false), call(false, false), call(false, true), call(true, true)],
        );
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].settled, vec![0, 1]);
        assert_eq!(segments[0].live, vec![2, 3]);
    }

    #[test]
    fn an_empty_timeline_groups_nothing() {
        assert_eq!(group_segments(&[], &[]), Vec::<Segment>::new());
    }

    #[test]
    fn the_block_title_counts_in_the_right_number() {
        assert_eq!(actions_title(1), "1 action");
        assert_eq!(actions_title(12), "12 actions");
    }

    /// Only the newest block is open without being asked: it is where the run
    /// still is. A click decides either way, and keeps deciding after the next
    /// answer arrives.
    #[test]
    fn the_block_after_the_newest_answer_is_the_open_one() {
        assert!(is_open(false, false, true), "newest opens by default");
        assert!(!is_open(false, false, false), "an older block stays folded");
        assert!(!is_open(false, true, true), "closing the newest block keeps it closed");
        assert!(is_open(true, false, false), "opening an old block keeps it open");
    }

    /// Two files written five times between them delivered two files, and the
    /// rows kept are each file's last write, because that is what is on disk.
    #[test]
    fn a_rewritten_file_keeps_one_row() {
        let paths = ["a.md", "b.md", "a.md", "a.md", "b.md"];
        assert_eq!(unique_writes(&paths), vec![3, 4]);
    }

    #[test]
    fn one_write_of_one_file_needs_no_collapsing() {
        assert_eq!(unique_writes(&["a.md"]), vec![0]);
        assert_eq!(unique_writes(&[]), Vec::<usize>::new());
    }

    /// A call that named no file cannot be folded into another call by that rule,
    /// so each of them keeps its own row.
    #[test]
    fn a_write_without_a_path_stands_alone() {
        assert_eq!(unique_writes(&["", "", "a.md"]), vec![0, 1, 2]);
    }

    /// In a block that holds both kinds, the commands are never collapsed with
    /// each other and a rewritten file still keeps the row of its last write, in
    /// the order the calls happened.
    #[test]
    fn a_mixed_block_keeps_every_command_and_one_row_per_file() {
        let shapes = [call(false, false), call(true, false), call(false, false), call(true, false)];
        let paths = ["", "a.md", "", "a.md"];
        assert_eq!(unique_actions(&shapes, &paths), vec![0, 2, 3]);
        // A read names a file too, but only writes collapse.
        let reads = [call(false, false), call(false, false)];
        assert_eq!(unique_actions(&reads, &["x.rs", "x.rs"]), vec![0, 1]);
    }

    #[test]
    fn a_row_starts_flat_and_takes_its_detail_by_builder() {
        let row = GroupRow::new("PY".into(), "src/a.py".into(), "created".into());
        assert_eq!((row.adds, row.dels, row.failed), (0, 0, false));
        let row = GroupRow::new("CMD".into(), "cargo test".into(), "Failed".into())
            .change(4, 1)
            .failed();
        assert_eq!((row.adds, row.dels, row.failed), (4, 1, true));
    }

    /// A file call is identified by its name, not by where it lives: the block
    /// already says which run wrote it, and the row's click reveals the path.
    #[test]
    fn a_file_call_shows_only_its_name() {
        assert_eq!(file_name("notes/attention-check.txt"), "attention-check.txt");
        assert_eq!(file_name("D:\\fennec-models\\qwen36-35b\\tokenizer.json"), "tokenizer.json");
        assert_eq!(file_name("web/"), "web");
        // The path is the row's business when the call carried one; the title is
        // only consulted when it did not, and then a path with spaces survives.
        assert_eq!(short_text("notes/todo list.md", "Edit file whatever"), "todo list.md");
        assert_eq!(short_text("", "READ · Read file src/gui_main.rs"), "gui_main.rs");
        assert_eq!(short_text("", "Write file notes/plan/2026-11-18 draft.md"), "2026-11-18 draft.md");
    }

    /// Seven words is enough to tell `cargo test` from `git push`; the rest of a
    /// command line is noise in a list of calls, and the ellipsis says so.
    #[test]
    fn a_command_call_keeps_seven_words() {
        assert_eq!(
            short_text("", "Run command cargo build --release --features gui --bin fennec-agent-gui --offline"),
            "Run command cargo build --release --features gui …"
        );
        // Exactly the limit is not cut, and a short command keeps its own text.
        assert_eq!(
            short_text("", "a b c d e f g"),
            "a b c d e f g"
        );
        assert_eq!(short_text("", "Run command ls -la"), "Run command ls -la");
        assert_eq!(short_text("", "CMD · Repository status"), "Repository status");
    }

    #[test]
    fn a_search_or_a_directory_is_not_a_file_call() {
        assert_eq!(called_file("Search code row_label"), None);
        assert_eq!(called_file("Inspect repository"), None);
        assert_eq!(called_file("Read file src/a.rs").as_deref(), Some("src/a.rs"));
        assert_eq!(called_file("Read file "), None, "a call naming no file");
    }

    /// The expanded row leads with the whole call only when something was cut, so
    /// a short command does not print itself twice.
    #[test]
    fn the_expanded_text_prepends_only_what_was_shortened() {
        assert_eq!(reveal_text("ls -la", "ls -la", "4 entries"), "4 entries");
        assert_eq!(reveal_text("app.js", "src/main/app.js", ""), "src/main/app.js");
        assert_eq!(
            reveal_text("app.js", "src/main/app.js", "12 lines"),
            "src/main/app.js\n\n12 lines"
        );
    }
}
