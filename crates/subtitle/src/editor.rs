//! The model of PotPlayer's subtitle browser and editor (Browse/Edit/Sync
//! Subtitles, Alt+E) and of Input Subtitles (Alt+P): the cues with their
//! times, the cue at the playback time, editing text and timing, adding,
//! deleting, splitting and joining cues, moving a selection in time, undo
//! and redo, and saving.

use crate::{Cue, Document, Format};

/// How many edits undo goes back.
const HISTORY: usize = 100;

/// A document being edited.
#[derive(Debug, Clone)]
pub struct Editor {
    document: Document,
    undo: Vec<Vec<Cue>>,
    redo: Vec<Vec<Cue>>,
    dirty: bool,
}

impl Editor {
    /// Edits `document`, its cues ordered by start.
    #[must_use]
    pub fn new(mut document: Document) -> Editor {
        document.sort();
        Editor {
            document,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
        }
    }

    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    #[must_use]
    pub fn cues(&self) -> &[Cue] {
        &self.document.cues
    }

    /// Edited since it was made or last saved.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// The cue shown at `ms`, or else the next one: where the browser
    /// scrolls to while the file plays.
    #[must_use]
    pub fn cue_at(&self, ms: i64) -> Option<usize> {
        self.document
            .cue_at(ms)
            .or_else(|| self.document.next_cue(ms))
    }

    /// Replaces a cue's text; `false` for an index out of range.
    pub fn set_text(&mut self, index: usize, text: &str) -> bool {
        if index >= self.cues().len() {
            return false;
        }
        self.edit(|cues| text.clone_into(&mut cues[index].text));
        true
    }

    /// Sets a cue's times, the end no earlier than the start and neither
    /// below 0; the cues are ordered by start again. The cue's new index.
    pub fn set_times(&mut self, index: usize, start_ms: i64, end_ms: i64) -> Option<usize> {
        if index >= self.cues().len() {
            return None;
        }
        let start_ms = start_ms.max(0);
        let end_ms = end_ms.max(start_ms);
        let mut moved = index;
        self.edit(|cues| {
            let mut cue = cues.remove(index);
            cue.start_ms = start_ms;
            cue.end_ms = end_ms;
            moved = insertion_point(cues, start_ms);
            cues.insert(moved, cue);
        });
        Some(moved)
    }

    /// Adds `cue` where its start puts it (after cues that start at the
    /// same time); its index.
    pub fn insert(&mut self, cue: Cue) -> usize {
        let at = insertion_point(self.cues(), cue.start_ms);
        self.edit(|cues| cues.insert(at, cue));
        at
    }

    /// Adds an empty cue after `index` (at the start for `None`), starting
    /// where that cue ends and lasting `length_ms`; its index.
    pub fn insert_after(&mut self, index: Option<usize>, length_ms: i64) -> usize {
        let start_ms = index
            .and_then(|at| self.cues().get(at))
            .map_or(0, |cue| cue.end_ms);
        let cue = Cue::new(start_ms, start_ms + length_ms.max(0), String::new());
        self.insert(cue)
    }

    /// Deletes a cue; `false` for an index out of range.
    pub fn delete(&mut self, index: usize) -> bool {
        if index >= self.cues().len() {
            return false;
        }
        self.edit(|cues| {
            cues.remove(index);
        });
        true
    }

    /// Splits a cue at `at_ms`, inside it: the first part ends there, the
    /// second starts there. Lines are shared between the parts (the first
    /// gets the first half), one line is split at the space nearest its
    /// middle. `false` when `at_ms` is not inside the cue.
    pub fn split(&mut self, index: usize, at_ms: i64) -> bool {
        let Some(cue) = self.cues().get(index) else {
            return false;
        };
        if at_ms <= cue.start_ms || at_ms >= cue.end_ms {
            return false;
        }
        let (first, second) = split_text(&cue.text);
        self.edit(|cues| {
            let mut tail = cues[index].clone();
            cues[index].end_ms = at_ms;
            cues[index].text = first;
            tail.start_ms = at_ms;
            tail.text = second;
            cues.insert(index + 1, tail);
        });
        true
    }

    /// Joins a cue with the next one: it lasts up to the later end and
    /// takes the next one's lines after its own. `false` when there is no
    /// next cue.
    pub fn join_next(&mut self, index: usize) -> bool {
        if index + 1 >= self.cues().len() {
            return false;
        }
        self.edit(|cues| {
            let next = cues.remove(index + 1);
            let cue = &mut cues[index];
            cue.end_ms = cue.end_ms.max(next.end_ms);
            if cue.text.is_empty() {
                cue.text = next.text;
            } else if !next.text.is_empty() {
                cue.text = format!("{}\n{}", cue.text, next.text);
            }
        });
        true
    }

    /// Moves the cues of `indices` by `delay_ms` (none below 0); the cues
    /// are ordered by start again. `false` when no index is in range.
    pub fn shift(&mut self, indices: &[usize], delay_ms: i64) -> bool {
        let count = self.cues().len();
        if !indices.iter().any(|at| *at < count) {
            return false;
        }
        self.edit(|cues| {
            for &at in indices.iter().filter(|at| **at < count) {
                let cue = &mut cues[at];
                let length = cue.end_ms - cue.start_ms;
                cue.start_ms = (cue.start_ms + delay_ms).max(0);
                cue.end_ms = cue.start_ms + length;
            }
            cues.sort_by_key(|cue| cue.start_ms);
        });
        true
    }

    /// Takes the last edit back; `false` when there is none.
    pub fn undo(&mut self) -> bool {
        let Some(cues) = self.undo.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.document.cues, cues);
        self.redo.push(current);
        self.dirty = true;
        true
    }

    /// Makes the last edit undone again; `false` when there is none.
    pub fn redo(&mut self) -> bool {
        let Some(cues) = self.redo.pop() else {
            return false;
        };
        let current = std::mem::replace(&mut self.document.cues, cues);
        self.undo.push(current);
        self.dirty = true;
        true
    }

    /// The document as a file of `format` ([`Document::to_bytes`]); it
    /// counts as saved.
    pub fn save_bytes(&mut self, format: Format) -> Vec<u8> {
        self.dirty = false;
        self.document.to_bytes(format)
    }

    /// One edit: the cues before it go to the undo history.
    fn edit(&mut self, change: impl FnOnce(&mut Vec<Cue>)) {
        self.undo.push(self.document.cues.clone());
        if self.undo.len() > HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
        change(&mut self.document.cues);
        self.dirty = true;
    }
}

/// Where a cue starting at `start_ms` goes: after every cue that starts no
/// later.
fn insertion_point(cues: &[Cue], start_ms: i64) -> usize {
    cues.partition_point(|cue| cue.start_ms <= start_ms)
}

/// A cue's text in two: the first half of its lines and the rest, or one
/// line at the space nearest its middle.
fn split_text(text: &str) -> (String, String) {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() > 1 {
        let half = lines.len().div_ceil(2);
        return (lines[..half].join("\n"), lines[half..].join("\n"));
    }
    let middle = text.len() / 2;
    let space = text
        .char_indices()
        .filter(|(_, c)| *c == ' ')
        .min_by_key(|(at, _)| at.abs_diff(middle))
        .map(|(at, _)| at);
    match space {
        Some(at) => (text[..at].to_owned(), text[at + 1..].to_owned()),
        None => (text.to_owned(), String::new()),
    }
}
