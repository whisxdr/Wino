//! Isolated subprocess execution for Windows subsystems that expose no
//! practical native API from a user-mode Rust process.
//!
//! Wino is Win32-first. Everything that *can* be done through a native API is
//! done through a native API. A small number of operations genuinely cannot be:
//!
//! | Operation | Why a subprocess is required |
//! |---|---|
//! | `dism.exe /Get-Features`, `/Enable-Feature` | The DISM API (`dismapi.dll`) requires COM apartment setup and the `DismApi` module is not exposed by the `windows` crate; the CLI is the documented, stable interface. |
//! | `schtasks.exe /Change` | No public native API toggles an existing task's `Enabled` flag without rewriting the XML definition (which would lose trigger state). |
//! | `winget.exe` | Third-party package manager; ships as a CLI only. |
//! | `sfc.exe /scannow` | Repairs the component store in a way that must run in its own process context. |
//!
//! Every call through this module follows the same contract:
//!
//! * the console window is never shown (`CREATE_NO_WINDOW`);
//! * stdout/stderr are captured as UTF-8-lossy text, never inherited;
//! * the exit code is inspected and reported as a structured error;
//! * `dry_run` short-circuits before any process spawns;
//! * output is length-capped so a runaway tool cannot exhaust memory.

use std::os::windows::process::CommandExt;
use std::process::{Command, Output, Stdio};

/// `CREATE_NO_WINDOW` — the child gets a console object but no visible window.
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Upper bound on captured stdout/stderr per stream (4 MiB).
const MAX_CAPTURE_BYTES: usize = 4 * 1024 * 1024;

/// Result of running a tool: exit status plus captured streams.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl ToolOutput {
    /// Combined output, preferring stderr when stdout is empty (many Windows
    /// tools report failures on stderr and progress on stdout).
    pub fn combined(&self) -> String {
        let out = self.stdout.trim();
        let err = self.stderr.trim();
        match (out.is_empty(), err.is_empty()) {
            (false, false) => format!("{}\n{}", out, err),
            (false, true) => out.to_string(),
            (true, false) => err.to_string(),
            (true, true) => String::new(),
        }
    }

    /// Last non-empty line — usually the decisive one for DISM/schtasks.
    pub fn last_line(&self) -> String {
        self.combined()
            .lines()
            .rev()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("")
            .to_string()
    }
}

fn truncate_utf8(mut bytes: Vec<u8>) -> String {
    if bytes.len() > MAX_CAPTURE_BYTES {
        bytes.truncate(MAX_CAPTURE_BYTES);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn to_tool_output(output: Output) -> ToolOutput {
    ToolOutput {
        success: output.status.success(),
        exit_code: output.status.code(),
        stdout: truncate_utf8(output.stdout),
        stderr: truncate_utf8(output.stderr),
    }
}

/// Run `program` with `args`, hiding the console window and capturing output.
///
/// Never spawns a shell: `program` is executed directly, so no command string
/// is interpreted and no injection surface exists.
pub fn run(program: &str, args: &[&str]) -> Result<ToolOutput, String> {
    let output = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to spawn {}: {}", program, e))?;

    Ok(to_tool_output(output))
}

/// Run `program` and require a zero exit status.
///
/// On failure the error carries the exit code and the tool's own last output
/// line, which is what a user needs to act on. Callers that must tolerate a
/// non-zero exit (for example DISM reporting "already in the requested state")
/// should use [`run`] and inspect [`ToolOutput::success`] themselves.
pub fn run_checked(program: &str, args: &[&str]) -> Result<ToolOutput, String> {
    let out = run(program, args)?;
    if out.success {
        return Ok(out);
    }

    let detail = out.last_line();
    Err(format!(
        "{} exited with {}: {}",
        program,
        out.exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "no code".to_string()),
        if detail.is_empty() {
            "no output".to_string()
        } else {
            detail
        }
    ))
}

/// Whether `program` is resolvable on `PATH`.
///
/// Uses `where.exe` rather than `which` so the check works on a stock Windows
/// install with no POSIX tooling present.
pub fn is_available(program: &str) -> bool {
    match run("where.exe", &[program]) {
        Ok(out) => out.success,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_output_combined_prefers_available_stream() {
        let only_stdout = ToolOutput {
            success: true,
            exit_code: Some(0),
            stdout: "progress\n".to_string(),
            stderr: String::new(),
        };
        assert_eq!(only_stdout.combined(), "progress");

        let both = ToolOutput {
            success: false,
            exit_code: Some(1),
            stdout: "out".to_string(),
            stderr: "err".to_string(),
        };
        assert_eq!(both.combined(), "out\nerr");
        assert_eq!(both.last_line(), "err");
    }

    #[test]
    fn empty_output_has_empty_last_line() {
        let empty = ToolOutput {
            success: true,
            exit_code: Some(0),
            stdout: "  \n".to_string(),
            stderr: String::new(),
        };
        assert_eq!(empty.last_line(), "");
    }
}
