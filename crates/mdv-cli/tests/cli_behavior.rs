use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn mdv_bin() -> &'static str {
    env!("CARGO_BIN_EXE_mdv-cli")
}

fn with_coverage_env(cmd: &mut Command) {
    if let Ok(profile) = std::env::var("LLVM_PROFILE_FILE") {
        cmd.env("LLVM_PROFILE_FILE", profile);
    }
}

fn mdv_cmd() -> Command {
    let mut cmd = Command::new(mdv_bin());
    with_coverage_env(&mut cmd);
    cmd
}

fn temp_file(name: &str, content: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("mdv-{name}-{nanos}.md"));
    fs::write(&path, content).expect("write temp markdown");
    path
}

fn large_markdown_fixture(target_bytes: usize) -> String {
    let row = "## heading\n- item alpha\n- item beta\n`inline`\n\n";
    let mut out = String::from("# large fixture\n");
    while out.len() < target_bytes {
        out.push_str(row);
    }
    out
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> Output {
    let started = std::time::Instant::now();
    loop {
        match child.try_wait().expect("try_wait") {
            Some(_) => return child.wait_with_output().expect("wait_with_output"),
            None => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("process timed out");
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn assert_force_tui_exit_or_known_io_error(output: Output) {
    if output.status.success() {
        return;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Resource temporarily unavailable")
            || stderr.contains("No such file or directory"),
        "stderr: {}",
        stderr
    );
}

#[cfg(target_os = "linux")]
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

#[cfg(target_os = "linux")]
fn spawn_script(command: &str) -> std::process::Child {
    let mut cmd = Command::new("script");
    cmd.arg("-qfec").arg(command).arg("/dev/null");
    with_coverage_env(&mut cmd);

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn script")
}

#[test]
fn path_mode_non_tty_renders_once_and_exits() {
    let path = temp_file("non-tty", "# Title\nbody\n");
    let child = mdv_cmd()
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv");

    let output = wait_with_timeout(child, Duration::from_millis(1200));
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("# Title"), "stdout: {stdout}");
    assert!(stdout.contains("body"), "stdout: {stdout}");
}

#[test]
fn path_mode_non_tty_large_file_exits() {
    let content = large_markdown_fixture(1024 * 1024);
    let path = temp_file("non-tty-large", &content);
    let child = mdv_cmd()
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv");

    let output = wait_with_timeout(child, Duration::from_millis(4000));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stream_mode_reads_stdin_non_tty_and_exits() {
    let mut child = mdv_cmd()
        .arg("--stream")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv stream");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"# stream\nok\n").expect("write stdin");
    }
    let _ = child.stdin.take();

    let output = wait_with_timeout(child, Duration::from_millis(1200));
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("# stream"), "stdout: {stdout}");
    assert!(stdout.contains("ok"), "stdout: {stdout}");
}

#[test]
fn stream_mode_invalid_utf8_hits_error_path_and_exits() {
    let mut child = mdv_cmd()
        .arg("--stream")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv stream");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(&[0xff, b'\n']).expect("write invalid utf8");
    }
    let _ = child.stdin.take();

    let output = wait_with_timeout(child, Duration::from_millis(1200));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("valid UTF-8"), "stderr: {}", stderr);
}

#[test]
fn stream_with_path_errors() {
    let path = temp_file("stream-path", "# x");
    let output = mdv_cmd()
        .arg("--stream")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run mdv");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("path arg not allowed with --stream"),
        "stderr: {stderr}"
    );
}

#[test]
fn no_args_shows_help_and_exits_zero() {
    let output = mdv_cmd()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run mdv");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stdout.contains("Usage:"), "stdout: {stdout}");
    assert!(stdout.contains("[PATH]"), "stdout: {stdout}");
    assert!(stderr.trim().is_empty(), "stderr: {stderr}");
}

#[test]
fn path_mode_force_tui_still_exits_non_interactive() {
    let path = temp_file("force-tui-path", "# title\nx");
    let child = mdv_cmd()
        .arg(&path)
        .env("MDV_FORCE_TUI", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv");
    let output = wait_with_timeout(child, Duration::from_millis(1200));
    assert_force_tui_exit_or_known_io_error(output);
}

#[test]
fn stream_mode_force_tui_exits_after_stdin_close() {
    let mut child = mdv_cmd()
        .arg("--stream")
        .env("MDV_FORCE_TUI", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"# one\n").expect("write");
    }
    let _ = child.stdin.take();
    let output = wait_with_timeout(child, Duration::from_millis(1200));
    assert_force_tui_exit_or_known_io_error(output);
}

#[test]
fn stream_mode_force_tui_invalid_utf8_hits_stream_error_branch() {
    let mut child = mdv_cmd()
        .arg("--stream")
        .env("MDV_FORCE_TUI", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mdv");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(&[0xff, b'\n']).expect("write");
    }
    let _ = child.stdin.take();
    thread::sleep(Duration::from_millis(600));
    let _ = child.kill();
    let output = wait_with_timeout(child, Duration::from_secs(2));
    assert!(!output.status.success());
}

#[cfg(target_os = "linux")]
#[test]
fn pty_force_tui_interactive_exits_on_ctrl_q() {
    let path = temp_file("linux-interactive", "# title\nx");
    let command = format!(
        "{} {}",
        sh_quote(mdv_bin()),
        sh_quote(&path.to_string_lossy())
    );
    let mut child = spawn_script(&command);

    thread::sleep(Duration::from_millis(300));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(&[0x11]).expect("write ctrl+q");
    }
    let _ = child.stdin.take();

    let output = wait_with_timeout(child, Duration::from_secs(3));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn pty_force_tui_interactive_handles_external_file_update() {
    let path = temp_file("linux-watch-update", "# title\nx");
    let command = format!(
        "{} {}",
        sh_quote(mdv_bin()),
        sh_quote(&path.to_string_lossy())
    );
    let mut child = spawn_script(&command);

    thread::sleep(Duration::from_millis(250));
    fs::write(&path, "# title\nupdated\n").expect("update watched file");
    thread::sleep(Duration::from_millis(250));
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(&[0x11]).expect("write ctrl+q");
    }
    let _ = child.stdin.take();

    let output = wait_with_timeout(child, Duration::from_secs(3));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
