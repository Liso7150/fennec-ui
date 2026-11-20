//! Shared grouping of a run's tool calls into the blocks every Fennec surface
//! shows: one expandable block per stretch between two answers, holding every
//! action of that stretch — the files it wrote and the tools it ran, with the
//! run's own thinking sitting in the place it thought it. A call the run is
//! still inside stays on its own so its output can be watched live.
//!
//! No settled block is open on its own: it starts folded and the user's click is
//! what unfolds it. The block the run is still inside is the exception — it is
//! open while the run is in it, because watching the work happen is the whole
//! point of that one, and the answer that ends the run folds it back.
//!
//! The app owns what a call *is* (the shell keeps a `ToolEvent` per call,
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

/// Whether the stretch that ends at `end` is the run's present tense. Only the
/// last stretch in the timeline can be: an older one whose answer never landed is
/// still history, and ticking it as well would put a second live clock on screen
/// the moment the user started another run in the same task. Being live is also
/// what makes a block open without a click — see [`block_title`] for the other
/// half of that rule, the words that replace the clock when the answer lands.
pub fn live_stretch(working: bool, end: usize, len: usize) -> bool {
    working && end == len
}

/// Whether the block is drawn open. A run the user is still watching shows its
/// work as it happens; a finished stretch is a record, so it folds itself and
/// says what it did on one line. `override_` is the user's own last click on this
/// block, which outranks the default in both directions: a run they folded stays
/// folded, a settled one they opened stays open.
pub fn block_open(live: bool, override_: Option<bool>) -> bool {
    override_.unwrap_or(live)
}

/// What a row was, so a settled block can say what the run did in the past tense
/// instead of only how many rows it holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ActionKind {
    /// The model's own reasoning. The block carries it in the place it was
    /// thought, but it is not work the run delivered, so it is in none of the
    /// counted buckets — see [`block_title`].
    Thought,
    Read,
    Edit,
    Run,
    Search,
    #[default]
    Other,
}

/// Which kind a call was, from the words [`crate::attention::describe_call`] gave
/// it, so every surface counts the same call the same way. The category badge is
/// stripped first because `READ · Read file x` and `Read file x` are one call.
pub fn kind_of(title: &str, wrote: bool) -> ActionKind {
    let body = strip_badge(title).to_ascii_lowercase();
    match body.split_whitespace().next().unwrap_or("") {
        "read" => ActionKind::Read,
        "write" | "edit" | "delete" => ActionKind::Edit,
        "run" => ActionKind::Run,
        // `Inspect repository web/` is what `describe_call` makes of a directory
        // listing: the run was looking for something, not changing it.
        "search" | "semantic" | "inspect" => ActionKind::Search,
        _ if wrote => ActionKind::Edit,
        _ => ActionKind::Other,
    }
}

/// How many rows a block shows before the rest go behind scrolling. Enough to
/// follow a live run without the timeline becoming the run's transcript.
pub const VISIBLE_ROWS: usize = 8;

/// Where each in-flight call's live progress line stands, keyed by the loop's
/// own `call_id`. One line per call is the contract: a newer report replaces the
/// older one instead of adding a row, and the line leaves when the call it
/// belongs to returns.
///
/// A `progress` step that reports no text is not stored at all, so a malformed
/// step cannot put an empty row on screen or drop one that is already there.
#[derive(Clone, Debug, Default)]
pub struct ProgressLines(std::collections::HashMap<String, String>);

impl ProgressLines {
    /// The line for one in-flight call, if it has reported one.
    pub fn get(&self, call_id: &str) -> Option<&str> {
        self.0.get(call_id).map(String::as_str)
    }

    /// How many calls currently have a line.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Forget every line: the run they belonged to is over, and a stale line
    /// would claim work that stopped happening.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Forget one call's line — its card closed, or this window stopped
    /// watching the run it was in.
    pub fn remove(&mut self, call_id: &str) -> Option<String> {
        self.0.remove(call_id)
    }

    /// Fold one `progress` step into the lines, taking the words from the fields
    /// of a `/api/agent/poll` step. Returns whether the step said anything
    /// usable: a step with no `call_id` or no `text` belongs to no card and
    /// changes nothing, which is the same outcome as an unknown step type. A
    /// report that replaces a line never grows the count of a running block —
    /// see [`ProgressLines`] for why that is the rule here.
    pub fn record_step(&mut self, step: &serde_json::Value) -> bool {
        let Some(call_id) = step["call_id"].as_str().filter(|id| !id.is_empty()) else {
            return false;
        };
        let Some(text) = step["text"].as_str() else {
            return false;
        };
        self.record(call_id, progress_line(text, positive(step, "step"), positive(step, "of")))
    }

    /// Put one call's live line in place, replacing whatever it said before.
    /// Empty words report nothing, so they leave the line already on screen.
    pub fn record(&mut self, call_id: &str, line: impl Into<String>) -> bool {
        if call_id.is_empty() {
            return false;
        }
        let line = line.into();
        if line.is_empty() {
            return false;
        }
        self.0.insert(call_id.to_string(), line);
        true
    }

    /// The row a call's live line draws inside its block: the loop's sentence,
    /// dim, uncounted, and opening nothing on click.
    pub fn row(&self, call_id: &str, icon: &str) -> Option<GroupRow> {
        let line = self.get(call_id)?;
        Some(GroupRow::progress(icon, line))
    }

    /// Drop every line of calls this window no longer watches, keeping the ones
    /// in `live_ids`. A card that closed, or a run this window stopped following,
    /// takes its line with it: a stale sentence would claim work that already
    /// stopped happening.
    pub fn retain(&mut self, live_ids: impl Fn(&str) -> bool) {
        self.0.retain(|call_id, _| live_ids(call_id));
    }
}

/// A step counter the loop means as a position: a positive integer, and nothing
/// else. A float, a string numeral or a zero says the loop did not count.
fn positive(step: &serde_json::Value, key: &str) -> Option<u64> {
    step[key].as_u64().filter(|n| *n > 0)
}

/// The words a live line stores for one `progress` step: the loop's own sentence,
/// verbatim, and — when it counted both halves — a dim meter after them. Nothing
/// here is composed from `step`/`of`: the sentence is the backend's, so this only
/// adds the count it did not state. A missing half of the pair stays missing.
fn progress_line(text: &str, step: Option<u64>, of: Option<u64>) -> String {
    match (step, of) {
        (Some(s), Some(o)) => format!("{text} {s}/{o}"),
        _ => text.to_string(),
    }
}

/// A clock in `MM:SS`, the way the rest of the app spells elapsed time.
pub fn clock(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m:02}:{s:02}") }
}

/// The one line a block header carries. While the run is inside it the clock
/// ticks and the clock is the whole point; when the answer lands the clock is
/// gone and the block says, in the past tense, what the run did: the tools it
/// ran, the files it read, the files it wrote.
///
/// The three buckets are disjoint — a read and a write are not also counted as
/// tools — so the numbers add up to the calls the block held. A thought is in
/// none of them: it is what the run was thinking, not work it delivered. A block
/// that holds only thoughts says what it holds rather than that nothing happened.
pub fn block_title(rows: &[GroupRow], live: bool, elapsed_secs: u64) -> String {
    if live {
        return format!("Running · {}", clock(elapsed_secs));
    }
    let mut tools = 0usize;
    let mut reads = 0usize;
    let mut writes = 0usize;
    let mut thoughts = 0usize;
    for row in rows {
        match row.kind {
            ActionKind::Thought => thoughts += 1,
            ActionKind::Read => reads += 1,
            ActionKind::Edit => writes += 1,
            _ => tools += 1,
        }
    }
    // (count, verb, singular noun) in the order the line reads them; a zero
    // bucket says nothing rather than `0 tools`.
    let buckets = [(tools, "ran", "tool"), (reads, "read", "file"), (writes, "wrote", "file")];
    let parts: Vec<String> = buckets
        .iter()
        .filter(|(n, _, _)| *n > 0)
        .enumerate()
        .map(|(i, (n, verb, noun))| {
            let object = if *n == 1 { noun.to_string() } else { format!("{noun}s") };
            // Only the first phrase carries the line, so it takes the capital.
            let verb = if i == 0 { verb[..1].to_ascii_uppercase() + &verb[1..] } else { verb.to_string() };
            format!("{verb} {n} {object}")
        })
        .collect();
    if parts.is_empty() {
        // Nothing counted, but the stretch was not empty: the run thought
        // without acting, and the block holds that.
        return if thoughts > 0 { "Thinking".to_string() } else { "No calls".to_string() };
    }
    parts.join(" · ")
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
/// the app keeps its own click handlers, because only it knows whether a
/// row opens a file, reviews a diff, or has nothing behind it.
#[derive(Clone)]
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
    /// What the row was, which is what the settled block counts to build its
    /// title. A thought row carries `Thought` and is counted by no title.
    pub kind: ActionKind,
    /// The line that says what a still-running call is doing right now. It is
    /// one row per in-flight call, replaced by the next report of it, so it is
    /// never clickable and never counted — see [`PROGRESS_STATUS`].
    pub live: bool,
}

/// The status a live progress row carries. A call's own row says `Complete ·
/// 0.8s` when it lands; this row has no outcome to report, only the words the
/// loop chose while the call is still blocked.
pub const PROGRESS_STATUS: &str = "Running";

impl GroupRow {
    pub fn new(icon: String, label: String, status: String) -> Self {
        Self { icon, label, adds: 0, dels: 0, status, failed: false, kind: ActionKind::default(), live: false }
    }

    /// A row for one settled call: `path` is the file it named — a read names one
    /// too, so this is not only for writes — and `title` is how the surface spells
    /// the call. The row shows the shortened words; the call keeps its whole text
    /// for when the user opens the row.
    pub fn call(icon: String, path: &str, title: &str, status: String) -> Self {
        Self::new(icon, short_text(path, title), status).kind(kind_of(title, false))
    }

    /// One thought of the run, as a row of its block. The whole reasoning stays
    /// with the row and is what its click prints; the folded label carries its
    /// first [`ROW_WORDS`] words, so a paragraph of thinking reads as one line
    /// among the calls it happened between. A thought row has no status of its
    /// own: it returned nothing.
    pub fn thought(text: &str) -> Self {
        Self::new("💭".into(), fold_words(text), String::new()).kind(ActionKind::Thought)
    }

    /// The one line a call the run is still inside shows for what it is doing.
    /// `line` is the loop's own sentence with its meter already joined — see
    /// [`ProgressLines`] — which reaches the user unchanged. A progress row
    /// belongs to its call, so it takes that call's icon.
    pub fn progress(icon: &str, line: &str) -> Self {
        Self::new(icon.to_string(), line.to_string(), PROGRESS_STATUS.to_string())
            .kind(ActionKind::Other)
            .uncounted()
            .as_live()
    }

    /// Take this row out of the counted buckets, keeping whatever kind it was
    /// classified as. A progress row is the one use: it says what a call is
    /// doing, not what the run delivered, so the settled block's counts must not
    /// change when it passes through.
    pub fn uncounted(mut self) -> Self {
        self.kind = ActionKind::Thought;
        self
    }

    /// Mark this row as the live line of an in-flight call: drawn dim, with no
    /// click behind it, and replaced rather than added to by the next report.
    pub fn as_live(mut self) -> Self {
        self.live = true;
        self
    }

    pub fn kind(mut self, kind: ActionKind) -> Self {
        self.kind = kind;
        self
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

/// The first [`ROW_WORDS`] words of `text`, joined by single spaces, with an
/// ellipsis when there were more — so a folded row always says that it is
/// folded. A thought's newlines are its paragraphs and a row is one line, so
/// they read as spaces here too.
fn fold_words(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= ROW_WORDS {
        return words.join(" ");
    }
    format!("{} …", words[..ROW_WORDS].join(" "))
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
    fold_words(strip_badge(title))
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
        ActionKind, CallShape, GroupRow, PROGRESS_STATUS, ProgressLines, Segment, block_open,
        block_title, called_file, clock, file_name, group_segments, kind_of, live_stretch,
        reveal_text, short_text, unique_actions, unique_writes,
    };
    use serde_json::json;

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

    fn row(kind: ActionKind) -> GroupRow {
        GroupRow::new("⚙".into(), "x".into(), String::new()).kind(kind)
    }

    /// A settled block says what the run did, in one fixed order, so the same
    /// stretch never reads differently on two surfaces. The buckets are disjoint:
    /// a read is not also a tool.
    #[test]
    fn the_settled_block_says_what_the_run_did() {
        let rows = vec![row(ActionKind::Run), row(ActionKind::Read), row(ActionKind::Read), row(ActionKind::Edit)];
        assert_eq!(block_title(&rows, false, 0), "Ran 1 tool · read 2 files · wrote 1 file");
        assert_eq!(block_title(&[row(ActionKind::Read)], false, 0), "Read 1 file");
        assert_eq!(block_title(&[row(ActionKind::Edit)], false, 0), "Wrote 1 file");
        assert_eq!(block_title(&(0..3).map(|_| row(ActionKind::Search)).collect::<Vec<_>>(), false, 0), "Ran 3 tools");
        // A call the table does not name is still a tool the run ran, not a zero.
        assert_eq!(block_title(&[row(ActionKind::Other)], false, 0), "Ran 1 tool");
        // Row order does not move the phrases: tools, then reads, then writes,
        // whichever of them the stretch happened to do first.
        assert_eq!(
            block_title(&[row(ActionKind::Read), row(ActionKind::Run)], false, 0),
            "Ran 1 tool · read 1 file"
        );
    }

    /// While the run is inside the block the clock is the only thing that
    /// matters; the counts arrive with the answer.
    #[test]
    fn a_live_block_shows_the_clock_and_nothing_else() {
        let rows = vec![row(ActionKind::Read), row(ActionKind::Run)];
        assert_eq!(block_title(&rows, true, 83), "Running · 01:23");
        assert_eq!(block_title(&[], true, 0), "Running · 00:00");
    }

    /// A thought is not work the run delivered, so it is counted by no bucket. A
    /// block of only thoughts is not empty, and says so.
    #[test]
    fn a_thought_is_never_counted() {
        let rows = vec![GroupRow::thought("checking the trunk"), row(ActionKind::Read)];
        assert_eq!(block_title(&rows, false, 0), "Read 1 file");
        assert_eq!(block_title(&[GroupRow::thought("only thinking")], false, 0), "Thinking");
        assert_eq!(block_title(&[], false, 0), "No calls");
    }

    /// A paragraph of reasoning is one line among the calls it happened between,
    /// folded the way a long command is, and the whole thought stays with the row
    /// for the click to print.
    #[test]
    fn a_thought_folds_to_seven_words() {
        let row = GroupRow::thought("The trunk is dense here, so the expert union cannot pay.\nI will read the file instead.");
        assert_eq!(row.label, "The trunk is dense here, so the …");
        assert_eq!(row.kind, ActionKind::Thought);
        assert!(row.status.is_empty(), "a thought returned nothing, so it has no status");
        assert_eq!(GroupRow::thought("one two three").label, "one two three");
    }

    /// The work the run is still doing is on show; the record it leaves behind is
    /// folded, and the user's own click outranks that in both directions.
    #[test]
    fn a_live_block_is_open_and_a_settled_one_is_folded() {
        assert!(block_open(true, None), "the running stretch shows its work");
        assert!(!block_open(false, None), "a settled stretch folds itself");
        assert!(!block_open(true, Some(false)), "a run the user folded stays folded");
        assert!(block_open(false, Some(true)), "a record the user opened stays open");
    }

    #[test]
    fn the_clock_keeps_two_digit_minutes_and_rolls_the_hour() {
        assert_eq!(clock(9), "00:09");
        assert_eq!(clock(600), "10:00");
        assert_eq!(clock(3725), "1:02:05");
    }

    /// The kind comes off the words the call already carries, badge or no badge,
    /// because the desktop spells `READ · Read file x` where the PWA spells
    /// `Read file x`.
    #[test]
    fn a_call_is_classified_from_its_own_words() {
        assert_eq!(kind_of("READ · Read file src/a.rs", false), ActionKind::Read);
        assert_eq!(kind_of("Read file src/a.rs", false), ActionKind::Read);
        assert_eq!(kind_of("CMD · Run command cargo test", false), ActionKind::Run);
        assert_eq!(kind_of("Run python print(1)", false), ActionKind::Run);
        assert_eq!(kind_of("FIND · Search code todo", false), ActionKind::Search);
        assert_eq!(kind_of("Semantic attention", false), ActionKind::Search);
        // A directory listing is the run looking for something, not changing it.
        assert_eq!(kind_of("Inspect repository web/", false), ActionKind::Search);
        assert_eq!(kind_of("EDIT · Write file a.md", true), ActionKind::Edit);
        assert_eq!(kind_of("Delete file a.md", false), ActionKind::Edit);
        // A call the table does not name is still one action, not a silent zero.
        assert_eq!(kind_of("Repository status", false), ActionKind::Other);
    }

    /// One clock per run: the stretch that reaches the end of the timeline is the
    /// present tense, and everything before it is history even while the task is
    /// working — otherwise a second run would re-light the first run's block.
    #[test]
    fn only_the_last_stretch_ticks() {
        assert!(live_stretch(true, 5, 5), "the stretch at the end is the run");
        assert!(!live_stretch(true, 2, 5), "an older stretch is history");
        assert!(!live_stretch(false, 5, 5), "a finished run does not tick");
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
            short_text("", "Run command cargo build --release --features gui --bin fennec-desktop-gui --offline"),
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

    /// A long-running call says what it is doing while it is still blocked, and
    /// says it in the loop's own words: the sentence reaches the user unchanged,
    /// with the dim meter beside it when the loop counted its steps.
    #[test]
    fn a_progress_line_carries_the_loops_own_words() {
        let mut lines = ProgressLines::default();
        lines.record_step(&json!({"type": "progress", "name": "web_task", "call_id": "call_7",
                                  "text": "Deciding step 3 of 12", "step": 3, "of": 12, "i": 7, "t": 41.2}));
        let row = lines.row("call_7", "WEB").expect("the line is kept for its call");
        // Verbatim — not truncated, not re-cased, not rewritten from the counts.
        assert_eq!(row.label, "Deciding step 3 of 12 3/12");
        assert!(row.label.starts_with("Deciding step 3 of 12"), "the sentence comes first");
        assert_eq!(row.status, PROGRESS_STATUS);
        assert_eq!(row.icon, "WEB");
        assert!(row.live, "it is the live line of an in-flight call");
        // A meter only when both halves are there, and never one invented from
        // a lone number.
        let mut bare = ProgressLines::default();
        bare.record_step(&json!({"call_id": "c", "text": "Scrolling"}));
        assert_eq!(bare.row("c", "WEB").unwrap().label, "Scrolling");
        bare.record_step(&json!({"call_id": "c", "text": "Scrolling", "step": 4}));
        assert_eq!(bare.row("c", "WEB").unwrap().label, "Scrolling");
        bare.record_step(&json!({"call_id": "c", "text": "Scrolling", "of": 9}));
        assert_eq!(bare.row("c", "WEB").unwrap().label, "Scrolling");
        bare.record_step(&json!({"call_id": "c", "text": "Scrolling", "step": 4, "of": 9}));
        assert_eq!(bare.row("c", "WEB").unwrap().label, "Scrolling 4/9");
    }

    /// One visible line per in-flight call: the next report of the same call
    /// replaces the line that is on screen rather than stacking a second row.
    #[test]
    fn a_newer_report_replaces_the_line_it_is_about() {
        let mut lines = ProgressLines::default();
        lines.record_step(&json!({"call_id": "call_7", "text": "Deciding step 3 of 12", "step": 3, "of": 12}));
        lines.record_step(&json!({"call_id": "call_7", "text": "Typing into the search box"}));
        assert_eq!(lines.len(), 1, "one call, one line");
        assert_eq!(lines.get("call_7"), Some("Typing into the search box"));
        // Two calls running at once keep their own lines, and a settled call's
        // line goes with its card.
        lines.record_step(&json!({"call_id": "call_8", "text": "Waiting for the page"}));
        assert_eq!(lines.len(), 2);
        lines.remove("call_7");
        assert_eq!(lines.get("call_7"), None);
        assert_eq!(lines.get("call_8"), Some("Waiting for the page"));
        lines.clear();
        assert!(lines.is_empty());
    }

    /// A progress row is what a call is doing, not what the run delivered, so it
    /// sits in none of the counted buckets: the block's counts read the same with
    /// the line as without it.
    #[test]
    fn a_progress_row_is_never_counted() {
        let mut lines = ProgressLines::default();
        lines.record_step(&json!({"call_id": "call_7", "text": "Deciding step 3 of 12", "step": 3, "of": 12}));
        let live = lines.row("call_7", "WEB").unwrap();
        let quiet = vec![
            GroupRow::new("CMD".into(), "cargo test".into(), "Complete".into()).kind(ActionKind::Run),
            GroupRow::new("PY".into(), "a.py".into(), "created".into()).kind(ActionKind::Edit),
        ];
        let mut with = quiet.clone();
        with.push(live);
        assert_eq!(block_title(&quiet, true, 41), block_title(&with, true, 41));
        // Settled, the same holds: the line never became a third thing the run ran.
        assert_eq!(block_title(&quiet, false, 0), "Ran 1 tool · wrote 1 file");
        assert_eq!(block_title(&with, false, 0), "Ran 1 tool · wrote 1 file");
    }

    /// A live block keeps its clock while a call is reporting: the line says the
    /// run is working, it never settles or folds the block it belongs to.
    #[test]
    fn a_progress_line_keeps_its_block_running() {
        let mut lines = ProgressLines::default();
        lines.record_step(&json!({"call_id": "call_7", "text": "Deciding step 3 of 12", "step": 3, "of": 12}));
        let rows = vec![GroupRow::new("CMD".into(), "cargo test".into(), "Complete".into())
            .kind(ActionKind::Run), lines.row("call_7", "WEB").unwrap()];
        assert_eq!(block_title(&rows, true, 96), "Running · 01:36");
    }

    /// A `progress` step that names no call or brings no sentence reports
    /// nothing: it cannot open a row, and it must not clear the one already on
    /// screen.
    #[test]
    fn a_progress_step_that_reports_nothing_changes_nothing() {
        let mut lines = ProgressLines::default();
        lines.record_step(&json!({"call_id": "call_7", "text": "Deciding step 3 of 12"}));
        for orphan in [
            json!({"text": "no call id"}),
            json!({"call_id": "", "text": "empty call id"}),
            json!({"call_id": "call_8"}),
            json!({"call_id": "call_8", "text": null}),
            json!({"call_id": "call_8", "text": 7}),
            json!({"call_id": "call_8", "text": ["a list"]}),
        ] {
            assert!(!lines.record_step(&orphan), "{orphan} should be ignored");
        }
        assert_eq!(lines.len(), 1, "nothing was added");
        assert_eq!(lines.get("call_7"), Some("Deciding step 3 of 12"), "nothing was lost");
        assert_eq!(lines.get("call_8"), None);
        // And a line whose call never opened draws no row of its own.
        assert!(lines.row("call_9", "WEB").is_none());
    }

    /// The meter is joined to the sentence once, when the step arrives, and a
    /// sentence that carries its own numbers keeps all of them.
    #[test]
    fn a_meter_joins_only_what_the_loop_counted() {
        let mut lines = ProgressLines::default();
        lines.record_step(&json!({"call_id": "a", "text": "Reading src/timeline.rs"}));
        assert_eq!(lines.row("a", "READ").unwrap().label, "Reading src/timeline.rs");
        // A sentence that ends in its own slash-count keeps it, and the meter the
        // loop counted separately still prints after it.
        lines.record_step(&json!({"call_id": "b", "text": "Click 2 / 5", "step": 2, "of": 5}));
        assert_eq!(lines.get("b"), Some("Click 2 / 5 2/5"));
        lines.record_step(&json!({"call_id": "b", "text": "Ratio 1/4"}));
        assert_eq!(lines.row("b", "WEB").unwrap().label, "Ratio 1/4");
        // A zero-count report is not a position, so no meter is put there.
        lines.record_step(&json!({"call_id": "c", "text": "Starting up", "step": 0, "of": 12}));
        assert_eq!(lines.row("c", "WEB").unwrap().label, "Starting up");
        // Nor is a number the loop spelled as text or as a fraction.
        lines.record_step(&json!({"call_id": "c", "text": "Waiting", "step": "3", "of": 12}));
        assert_eq!(lines.row("c", "WEB").unwrap().label, "Waiting");
    }

    /// A line a surface hands over already built goes in unchanged, and one of
    /// its calls can be dropped when that card closes.
    #[test]
    fn a_line_can_be_recorded_and_dropped_directly() {
        let mut lines = ProgressLines::default();
        assert!(!lines.record("", "orphan"), "a line with no call belongs nowhere");
        assert!(!lines.record("c1", ""), "empty words report nothing");
        assert!(lines.record("c1", "Deciding step 3 of 12 3/12"));
        assert!(lines.record("c2", "Waiting for the page"));
        lines.retain(|id| id == "c2");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines.get("c1"), None);
        assert_eq!(lines.get("c2"), Some("Waiting for the page"));
    }
}
