//! Shared grouping of a run's tool calls into the two blocks every Fennec
//! surface shows: one "Written N files" for everything the run put on disk, one
//! "Ran N tools" for everything else it called, and nothing else — a call the
//! run is still inside stays on its own so its output can be watched live.
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

/// Where the calls of one timeline landed, as indexes back into the caller's own
/// list in the order they were called.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Groups {
    pub written: Vec<usize>,
    pub ran: Vec<usize>,
    /// The calls still open. These are never folded into a block.
    pub live: Vec<usize>,
}

/// A running call has not written anything and has not finished running tools,
/// so it reads on its own; the settled calls fold.
pub fn split_calls(calls: &[CallShape]) -> Groups {
    let mut groups = Groups::default();
    for (i, call) in calls.iter().enumerate() {
        if call.running {
            groups.live.push(i);
        } else if call.wrote {
            groups.written.push(i);
        } else {
            groups.ran.push(i);
        }
    }
    groups
}

/// The count line on the write block. A run that wrote one file still says so.
pub fn written_title(n: usize) -> String {
    format!("Written {n} file{}", if n == 1 { "" } else { "s" })
}

/// The count line on the call block, which covers every non-write call — a
/// command, a search, a read — so it counts tools, not shells.
pub fn ran_title(n: usize) -> String {
    format!("Ran {n} tool{}", if n == 1 { "" } else { "s" })
}

/// One line inside a block: what the call was, and what came of it. Pure data —
/// the shells keep their own click handlers, because only they know whether a
/// row opens a file, reviews a diff, or has nothing behind it.
pub struct GroupRow {
    /// The file's own badge for a write, the category icon for a call.
    pub icon: String,
    /// The path for a write, the call's one-line description otherwise.
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

#[cfg(test)]
mod tests {
    use crate::timeline::{CallShape, GroupRow, Groups, ran_title, split_calls, written_title};

    fn call(wrote: bool, running: bool) -> CallShape {
        CallShape { wrote, running }
    }

    /// The rule the three surfaces share: nothing that is still happening belongs
    /// in a block, because the count would be a promise.
    #[test]
    fn a_running_call_is_never_folded() {
        let groups = split_calls(&[
            call(true, false),
            call(false, false),
            call(false, true),
            call(true, true),
            call(false, false),
        ]);
        assert_eq!(groups.written, vec![0]);
        assert_eq!(groups.ran, vec![1, 4]);
        // Both running calls stay out, whether they are writes or not.
        assert_eq!(groups.live, vec![2, 3]);
    }

    #[test]
    fn an_empty_timeline_groups_nothing() {
        assert_eq!(split_calls(&[]), Groups::default());
    }

    /// One write is still "1 file", not "files"; the words are what the user reads.
    #[test]
    fn the_block_titles_count_in_the_right_number() {
        assert_eq!(written_title(1), "Written 1 file");
        assert_eq!(written_title(3), "Written 3 files");
        assert_eq!(ran_title(1), "Ran 1 tool");
        assert_eq!(ran_title(12), "Ran 12 tools");
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
}
