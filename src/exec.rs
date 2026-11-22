//! Process execution helpers.
//!
//! Every external command lazypkg runs goes through this module so that we get
//! consistent behaviour everywhere:
//! - stdin is never inherited in captured mode (a child can't hang waiting for input),
//! - output is captured instead of being written over the TUI,
//! - long-running commands can be killed after a timeout,
//! - privilege elevation is explicit (`needs_root`) and visible in `display()`,
//!   never a silent retry after a failure.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Timeout for "list installed packages" style queries.
pub const LIST_TIMEOUT: Duration = Duration::from_secs(120);

/// A command to run, optionally with elevated privileges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub needs_root: bool,
}

impl Cmd {
    pub fn new<S: AsRef<str>>(program: &str, args: &[S]) -> Self {
        Cmd {
            program: program.to_string(),
            args: args.iter().map(|a| a.as_ref().to_string()).collect(),
            needs_root: false,
        }
    }

    pub fn with_root(mut self, needs_root: bool) -> Self {
        self.needs_root = needs_root;
        self
    }

    /// True if running this command will go through sudo/doas/pkexec.
    pub fn will_elevate(&self) -> bool {
        self.needs_root && !is_root()
    }

    /// Human-readable command line, including the elevation tool if one will be used.
    pub fn display(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.will_elevate() {
            parts.push(elevation_tool().unwrap_or("sudo").to_string());
        }
        parts.push(self.program.clone());
        parts.extend(self.args.iter().map(|a| shell_quote(a)));
        parts.join(" ")
    }

    fn to_command(&self, interactive: bool) -> Result<Command> {
        if self.will_elevate() {
            let tool = elevation_tool().context(
                "this action needs root, but none of sudo, doas or pkexec was found on PATH",
            )?;
            let mut c = Command::new(tool);
            // In captured (non-interactive) mode sudo must not wait for a password
            // on a stdin it doesn't have; fail fast instead.
            if !interactive && tool == "sudo" {
                c.arg("-n");
            }
            c.arg(&self.program).args(&self.args);
            Ok(c)
        } else {
            let mut c = Command::new(&self.program);
            c.args(&self.args);
            Ok(c)
        }
    }
}

/// Output of a captured command.
#[derive(Debug)]
pub struct Captured {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Run a command with stdin closed and stdout/stderr captured.
/// If `timeout` elapses the child is killed and an error is returned.
pub fn run_captured(cmd: &Cmd, timeout: Option<Duration>) -> Result<Captured> {
    let mut child = cmd
        .to_command(false)?
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start `{}`", cmd.program))?;

    // Drain both pipes on their own threads so a chatty child can't deadlock
    // by filling a pipe buffer while we wait on it.
    let mut out_pipe = child.stdout.take().expect("stdout piped");
    let mut err_pipe = child.stderr.take().expect("stderr piped");
    let out_t = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = out_pipe.read_to_end(&mut buf);
        buf
    });
    let err_t = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = err_pipe.read_to_end(&mut buf);
        buf
    });

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if let Some(limit) = timeout {
            if start.elapsed() >= limit {
                let _ = child.kill();
                let _ = child.wait();
                bail!("`{}` timed out after {}s", cmd.display(), limit.as_secs());
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let stdout = String::from_utf8_lossy(&out_t.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err_t.join().unwrap_or_default()).into_owned();
    Ok(Captured {
        status,
        stdout,
        stderr,
    })
}

/// Run a command captured and fail with a readable error (including the tail of
/// stderr) if it exits non-zero.
pub fn run_checked(cmd: &Cmd) -> Result<()> {
    let out = run_captured(cmd, None)?;
    if !out.status.success() {
        bail!(
            "{}",
            failure_message(cmd, out.status, &out.stderr, &out.stdout)
        );
    }
    Ok(())
}

/// Run a command attached to the user's terminal (for the CLI, or with the TUI
/// suspended) so that progress output and password prompts work.
pub fn run_interactive(cmd: &Cmd) -> Result<()> {
    let status = cmd
        .to_command(true)?
        .status()
        .with_context(|| format!("failed to start `{}`", cmd.program))?;
    if !status.success() {
        bail!("`{}` failed ({})", cmd.display(), describe_status(status));
    }
    Ok(())
}

fn failure_message(cmd: &Cmd, status: ExitStatus, stderr: &str, stdout: &str) -> String {
    let source = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let tail: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let tail = tail[tail.len().saturating_sub(3)..].join(" | ");
    let mut msg = format!("`{}` failed ({})", cmd.display(), describe_status(status));
    if !tail.is_empty() {
        msg.push_str(": ");
        msg.push_str(&tail);
    }
    if source.contains("externally-managed-environment") {
        msg.push_str(" (hint: this Python is managed by your distro; use pipx or uv instead)");
    }
    msg
}

fn describe_status(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {}", code),
        None => "killed by signal".to_string(),
    }
}

/// Locate an executable on PATH without spawning a process.
pub fn which(bin: &str) -> Option<PathBuf> {
    if bin.contains('/') {
        let p = PathBuf::from(bin);
        return is_executable(&p).then_some(p);
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(bin))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

/// Whether the current process already runs as root.
#[cfg(unix)]
pub fn is_root() -> bool {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
pub fn is_root() -> bool {
    false
}

/// Whether the current user can write to `path`.
#[cfg(unix)]
pub fn is_writable(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c_path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: c_path is a valid NUL-terminated string for the duration of the call.
    unsafe { libc::access(c_path.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(not(unix))]
pub fn is_writable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
}

/// The first available privilege elevation tool.
pub fn elevation_tool() -> Option<&'static str> {
    ["sudo", "doas", "pkexec"]
        .into_iter()
        .find(|t| which(t).is_some())
}

fn shell_quote(arg: &str) -> String {
    let safe = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@+%,".contains(c));
    if safe {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_quotes_unsafe_args() {
        let cmd = Cmd::new("echo", &["hello world", "plain", "it's"]);
        assert_eq!(cmd.display(), r"echo 'hello world' plain 'it'\''s'");
    }

    #[test]
    fn captured_output_and_status() {
        let out = run_captured(
            &Cmd::new("sh", &["-c", "echo out; echo err >&2; exit 3"]),
            None,
        )
        .unwrap();
        assert_eq!(out.stdout.trim(), "out");
        assert_eq!(out.stderr.trim(), "err");
        assert_eq!(out.status.code(), Some(3));
    }

    #[test]
    fn stdin_is_closed() {
        // `cat` would block forever if stdin were inherited from a terminal.
        let out = run_captured(
            &Cmd::new("cat", &[] as &[&str]),
            Some(Duration::from_secs(5)),
        )
        .unwrap();
        assert!(out.status.success());
    }

    #[test]
    fn timeout_kills_child() {
        let start = Instant::now();
        let err = run_captured(
            &Cmd::new("sleep", &["10"]),
            Some(Duration::from_millis(200)),
        )
        .unwrap_err();
        assert!(err.to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn run_checked_reports_stderr_tail() {
        let err = run_checked(&Cmd::new("sh", &["-c", "echo boom >&2; exit 1"])).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("exit code 1"), "{msg}");
        assert!(msg.contains("boom"), "{msg}");
    }

    #[test]
    fn pep668_hint() {
        let err = run_checked(&Cmd::new(
            "sh",
            &[
                "-c",
                "echo 'error: externally-managed-environment' >&2; exit 1",
            ],
        ))
        .unwrap_err();
        assert!(err.to_string().contains("pipx or uv"));
    }

    #[test]
    fn which_finds_sh_and_rejects_missing() {
        assert!(which("sh").is_some());
        assert!(which("definitely-not-a-real-binary-xyz").is_none());
    }
}
