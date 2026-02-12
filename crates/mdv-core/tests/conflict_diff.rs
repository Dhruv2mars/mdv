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
    assert_eq!(conflict.hunks.len(), 1);
    assert_eq!(conflict.hunks[0].local_start, 1);
    assert_eq!(conflict.hunks[0].external_start, 1);
    assert_eq!(
        conflict.hunks[0].local_lines,
        vec!["two".to_string(), "three!".to_string()]
    );
    assert_eq!(
        conflict.hunks[0].external_lines,
        vec!["TWO".to_string(), "three".to_string(), "four".to_string()]
    );
}

#[test]
fn editor_cursor_accessor_reports_byte_index() {
    let mut editor = EditorBuffer::new("ab".into());
    editor.move_left();
    assert_eq!(editor.cursor(), 1);
}

#[test]
fn editor_save_to_path_success_and_error_paths() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();

    let ok_path = std::env::temp_dir().join(format!("mdv-core-save-ok-{nanos}.md"));
    let mut editor = EditorBuffer::new("save".into());
    editor.save_to_path(&ok_path).expect("save ok");
    let _ = std::fs::remove_file(&ok_path);

    let err_path = std::env::temp_dir().join(format!("mdv-core-save-dir-{nanos}"));
    std::fs::create_dir(&err_path).expect("mkdir");
    let err = editor.save_to_path(&err_path).expect_err("save err");
    assert!(!err.to_string().is_empty());
    let _ = std::fs::remove_dir(&err_path);
}
