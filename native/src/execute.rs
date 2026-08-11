//! Execution utilities.
//!
//! Provides [`ExecRunner`] for running compiled binaries or Python scripts
//! with piped stdin/stdout/stderr, configurable timeout, and kill-on-drop.

use serde::Serialize;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Result of executing a binary or script.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub killed: bool,
}

/// Errors from [`ExecRunner::execute`].
#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Runs compiled binaries or Python scripts with piped I/O and timeout.
///
/// # Behaviour
///
/// - If `binary_path` ends with `.py`, it is run as `python3 <path>`.
/// - Otherwise it is run directly as a binary (e.g. `./binary`).
///
/// Stdin is written and then dropped to signal EOF. Stdout and stderr are
/// captured. If the process does not exit within `timeout_secs`, it is killed
/// (via `kill_on_drop(true)`).
pub struct ExecRunner;

impl ExecRunner {
    /// Execute a binary or Python script with the given input.
    pub async fn execute(
        &self,
        binary_path: &str,
        input: &str,
        timeout_secs: u64,
    ) -> Result<ExecResult, ExecError> {
        let is_python = binary_path.ends_with(".py");

        let mut cmd = if is_python {
            let mut c = Command::new("python3");
            c.arg(binary_path);
            c
        } else {
            Command::new(binary_path)
        };

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        // Write input to stdin, then drop to signal EOF
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).await?;
            // Drop closes stdin, sending EOF to the child process
        }

        match tokio::time::timeout(Duration::from_secs(timeout_secs), child.wait_with_output()).await {
            Ok(output) => {
                let output = output?;
                Ok(ExecResult {
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: output.status.code(),
                    timed_out: false,
                    killed: false,
                })
            }
            Err(_elapsed) => {
                // Timeout: the future is dropped, which drops the Child.
                // kill_on_drop(true) sends SIGKILL and reaps the process.
                Ok(ExecResult {
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    timed_out: true,
                    killed: true,
                })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;
    use tempfile::TempDir;

    /// Check whether g++ is available on this system.
    fn has_gxx() -> bool {
        which::which("g++").is_ok()
    }

    /// Check whether python3 is available on this system.
    fn has_python3() -> bool {
        which::which("python3").is_ok()
    }

    /// Compile a C++ source file inside `dir` and return the binary path.
    fn compile_cpp(source: &str, dir: &TempDir) -> Option<std::path::PathBuf> {
        let src_path = dir.path().join("test.cpp");
        let bin_path = dir.path().join("test");
        std::fs::write(&src_path, source).ok()?;
        let output = StdCommand::new("g++")
            .arg(&src_path)
            .arg("-o")
            .arg(&bin_path)
            .output()
            .ok()?;
        if output.status.success() {
            Some(bin_path)
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------
    // C++ adder: reads two ints from stdin, prints their sum
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_cpp_adder() {
        if !has_gxx() {
            eprintln!("skipping test_cpp_adder: g++ not found");
            return;
        }

        let dir = TempDir::new().unwrap();
        let source = r#"
#include <iostream>
int main() {
    int a, b;
    std::cin >> a >> b;
    std::cout << (a + b) << std::endl;
    return 0;
}
"#;
        let bin = compile_cpp(source, &dir).expect("failed to compile adder.cpp");
        let runner = ExecRunner;
        let result = runner.execute(bin.to_str().unwrap(), "3 5\n", 5)
            .await
            .unwrap();

        assert_eq!(result.stdout.trim(), "8");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
        assert!(!result.killed);
    }

    // -----------------------------------------------------------------------
    // Infinite loop: verify timeout behaviour
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_infinite_loop_timeout() {
        if !has_gxx() {
            eprintln!("skipping test_infinite_loop_timeout: g++ not found");
            return;
        }

        let dir = TempDir::new().unwrap();
        let source = r#"
int main() {
    while (true) {}
    return 0;
}
"#;
        let bin = compile_cpp(source, &dir).expect("failed to compile infinite_loop.cpp");
        let runner = ExecRunner;
        let result = runner.execute(bin.to_str().unwrap(), "", 2)
            .await
            .unwrap();

        assert!(result.timed_out, "expected timed_out=true");
        assert!(result.killed, "expected killed=true");
        assert_eq!(result.exit_code, None);
    }

    // -----------------------------------------------------------------------
    // Python echo: reads stdin and writes it back to stdout
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_python_echo() {
        if !has_python3() {
            eprintln!("skipping test_python_echo: python3 not found");
            return;
        }

        let dir = TempDir::new().unwrap();
        let script_path = dir.path().join("echo.py");
        std::fs::write(
            &script_path,
            "import sys; sys.stdout.write(sys.stdin.read())",
        )
        .unwrap();

        let runner = ExecRunner;
        let result = runner.execute(script_path.to_str().unwrap(), "hello world\n", 5)
            .await
            .unwrap();

        assert_eq!(result.stdout, "hello world\n");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
        assert!(!result.killed);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_binary_not_found() {
        let runner = ExecRunner;
        let result = runner.execute("/nonexistent/binary", "", 5).await;
        assert!(result.is_err(), "expected Err for nonexistent binary");
    }

    #[tokio::test]
    async fn test_empty_input() {
        if !has_gxx() {
            eprintln!("skipping test_empty_input: g++ not found");
            return;
        }

        let dir = TempDir::new().unwrap();
        // A program that just exits 0 with no I/O
        let source = r#"int main() { return 0; }"#;
        let bin = compile_cpp(source, &dir).expect("failed to compile noop");
        let runner = ExecRunner;
        let result = runner.execute(bin.to_str().unwrap(), "", 5)
            .await
            .unwrap();

        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
    }
}