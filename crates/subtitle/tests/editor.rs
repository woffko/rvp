//! The subtitle browser and editor's model.

use rvp_subtitle::{Cue, Document, Editor, Format};

fn editor() -> Editor {
    let mut document = Document::new(Format::Srt);
    document.cues = vec![
        Cue::new(5000, 7000, "Second"),
        Cue::new(1000, 3000, "First line\nsecond line"),
        Cue::new(8000, 9000, "Third one"),
    ];
    Editor::new(document)
}

fn texts(editor: &Editor) -> Vec<&str> {
    editor.cues().iter().map(|cue| cue.text.as_str()).collect()
}

#[test]
fn the_cue_at_a_time_is_the_one_shown_or_the_next() {
    let editor = editor();
    assert_eq!(texts(&editor)[0], "First line\nsecond line");
    assert_eq!(editor.cue_at(1500), Some(0));
    assert_eq!(editor.cue_at(4000), Some(1));
    assert_eq!(editor.cue_at(8500), Some(2));
    assert_eq!(editor.cue_at(9500), None);
}

#[test]
fn edits_change_text_and_timing_and_keep_the_order() {
    let mut editor = editor();
    assert!(!editor.is_dirty());
    assert!(editor.set_text(1, "Changed"));
    assert!(editor.is_dirty());
    // Moving the second cue past the third reorders them.
    assert_eq!(editor.set_times(1, 9500, 9000), Some(2));
    assert_eq!(editor.cues()[2].start_ms, 9500);
    assert_eq!(editor.cues()[2].end_ms, 9500);
    assert_eq!(editor.set_times(7, 0, 1), None);

    let at = editor.insert_after(Some(0), 1500);
    assert_eq!(at, 1);
    assert_eq!(
        (editor.cues()[1].start_ms, editor.cues()[1].end_ms),
        (3000, 4500)
    );
    let at = editor.insert(Cue::new(500, 900, "Before all"));
    assert_eq!(at, 0);
    assert!(editor.delete(0));
    assert!(!editor.delete(9));

    assert!(editor.split(0, 2000));
    assert_eq!(texts(&editor)[..2], ["First line", "second line"]);
    assert_eq!(
        (editor.cues()[0].end_ms, editor.cues()[1].start_ms),
        (2000, 2000)
    );
    assert!(!editor.split(0, 5000));
    assert!(editor.join_next(0));
    assert_eq!(texts(&editor)[0], "First line\nsecond line");
    assert_eq!(editor.cues()[0].end_ms, 3000);

    // One line splits at the space nearest its middle.
    let last = editor.cues().len() - 1;
    let start = editor.cues()[last - 1].start_ms;
    assert!(editor.split(last - 1, start + 100));
    assert_eq!(texts(&editor)[last - 1..=last], ["Third", "one"]);

    assert!(editor.shift(&[0, 1], -2000));
    assert_eq!(editor.cues()[0].start_ms, 0);
    assert_eq!(editor.cues()[0].end_ms, 2000);
    assert!(!editor.shift(&[42], 10));
}

#[test]
fn undo_and_redo_walk_the_edits() {
    let mut editor = editor();
    let original: Vec<Cue> = editor.cues().to_vec();
    editor.set_text(0, "One");
    editor.delete(2);
    let edited: Vec<Cue> = editor.cues().to_vec();
    assert!(editor.undo());
    assert!(editor.undo());
    assert_eq!(editor.cues(), original.as_slice());
    assert!(!editor.undo());
    assert!(editor.redo());
    assert!(editor.redo());
    assert_eq!(editor.cues(), edited.as_slice());
    assert!(!editor.redo());
    // A new edit drops what could be redone.
    editor.undo();
    editor.set_text(0, "Other");
    assert!(!editor.can_redo());
}

#[test]
fn saving_writes_the_chosen_format_and_clears_the_dirty_flag() {
    let mut editor = editor();
    editor.set_text(1, "Saved");
    let bytes = editor.save_bytes(Format::Srt);
    assert!(!editor.is_dirty());
    let text = String::from_utf8(bytes).expect("utf-8");
    assert!(text.starts_with(
        "1\r\n00:00:01,000 --> 00:00:03,000\r\nFirst line\r\nsecond line\r\n\r\n2\r\n"
    ));
    let vtt = String::from_utf8(editor.save_bytes(Format::WebVtt)).expect("utf-8");
    assert!(vtt.contains("00:00:05.000 --> 00:00:07.000\nSaved\n"));
}
