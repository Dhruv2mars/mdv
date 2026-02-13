use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {} failed: {err}", path.display()))
}

#[test]
fn readmes_do_not_contain_removed_shortcuts() {
    let root = repo_root();
    let root_readme = read(&root.join("README.md"));
    let pkg_readme = read(&root.join("packages/cli/README.md"));

    for stale in ["Ctrl+/", "Alt+,", "Alt+.", "Ctrl+W", "split-view"] {
        assert!(!root_readme.contains(stale), "root README contains {stale}");
        assert!(!pkg_readme.contains(stale), "pkg README contains {stale}");
    }
}

#[test]
fn readmes_point_to_in_app_docs() {
    let root = repo_root();
    let root_readme = read(&root.join("README.md"));
    let pkg_readme = read(&root.join("packages/cli/README.md"));

    assert!(root_readme.contains("In-app docs"));
    assert!(root_readme.contains("Cmd+,/Ctrl+,"));
    assert!(pkg_readme.contains("In-app docs"));
    assert!(pkg_readme.contains("Cmd+,/Ctrl+,"));
}
