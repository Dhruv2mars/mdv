use mdv_core::{EditorBuffer, compute_conflict_hunks};

#[test]
fn compute_conflict_hunks_returns_changed_blocks_only() {
    let local = "a\nsame\nb\nc";
    let external = "a\nsame\nB\nc\nd";
    let hunks = compute_conflict_hunks(local, external);

    assert_eq!(hunks.len(), 2);

    let first = &hunks[0];
    assert_eq!(first.local_start, 2);
    assert_eq!(first.external_start, 2);
    assert_eq!(first.local_lines, vec!["b".to_string()]);
    assert_eq!(first.external_lines, vec!["B".to_string()]);

    let second = &hunks[1];
    assert_eq!(second.local_start, 4);
    assert_eq!(second.external_start, 4);
    assert!(second.local_lines.is_empty());
    assert_eq!(second.external_lines, vec!["d".to_string()]);
}

#[test]
fn editor_conflict_state_contains_block_hunks() {
    let mut editor = EditorBuffer::new("one\ntwo\nthree".into());
    editor.insert_char('!');
    editor.on_external_change("one\nTWO\nthree\nfour".into());

    let conflict = editor.conflict().expect("conflict");
    assert_eq!(conflict.hunks.len(), 2);
    assert_eq!(conflict.hunks[0].local_start, 1);
    assert_eq!(conflict.hunks[0].external_start, 1);
    assert_eq!(conflict.hunks[0].local_lines, vec!["two".to_string()]);
    assert_eq!(conflict.hunks[0].external_lines, vec!["TWO".to_string()]);
}
