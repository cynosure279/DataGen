// Compilation utilities — CompileRunner compiles user C++/C code via system compiler.

use serde::Serialize;
use std::fmt;
use std::io::Write;
use std::process::Stdio;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Result of a compilation attempt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompileResult {
    pub success: bool,
    pub binary_path: Option<String>,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Errors that can occur during compilation.
#[derive(Debug)]
pub enum CompileError {
    /// Wraps [`std::io::Error`] from file or process operations.
    Io(std::io::Error),
    /// The compiler process did not finish within the timeout.
    Timeout(u64),
    /// The specified compiler binary was not found on the system.
    CompilerNotFound(String),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Io(e) => write!(f, "IO error: {}", e),
            CompileError::Timeout(s) => write!(f, "Compilation timed out after {}s", s),
            CompileError::CompilerNotFound(p) => write!(f, "Compiler not found: {}", p),
        }
    }
}

impl std::error::Error for CompileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompileError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CompileError {
    fn from(e: std::io::Error) -> Self {
        CompileError::Io(e)
    }
}

/// Compiles source code using a system compiler.
///
/// Supports C, C++, and Python (passthrough).
pub struct CompileRunner;

impl CompileRunner {
    /// Compile `source` code in the given `language`.
    ///
    /// - `language = "python"` (case-insensitive): no compilation needed,
    ///   returns success with `binary_path = None`.
    /// - Otherwise: writes source to a tempfile and invokes `compiler_path`
    ///   with the provided `args`. Default timeout is 30 seconds.
    pub async fn compile(
        &self,
        source: &str,
        language: &str,
        compiler_path: &str,
        args: &[String],
    ) -> Result<CompileResult, CompileError> {
        // Python passthrough — no compilation needed.
        if language.eq_ignore_ascii_case("python") {
            return Ok(CompileResult {
                success: true,
                binary_path: None,
                stderr: String::new(),
                exit_code: Some(0),
            });
        }

        // Verify the compiler binary exists.
        if which::which(compiler_path).is_err() {
            return Err(CompileError::CompilerNotFound(compiler_path.to_string()));
        }

        // Determine source file extension.
        let extension = match language {
            "c" => "c",
            "cpp" | "c++" => "cpp",
            _ => "cpp",
        };

        // Write source to a tempfile (auto-cleaned on drop).
        let mut source_file = NamedTempFile::with_suffix(format!(".{}", extension))?;
        source_file.write_all(source.as_bytes())?;
        let source_path = source_file.path().to_str().unwrap().to_string();

        // Derive output binary path (same as source path without extension).
        let binary_path = source_file.path().with_extension("");
        let binary_path_str = binary_path.to_str().unwrap().to_string();

        // Build the compiler command.
        let mut cmd = Command::new(compiler_path);
        cmd.arg("-o")
            .arg(&binary_path_str)
            .arg(&source_path);
        for arg in args {
            cmd.arg(arg);
        }

        // Capture stderr for error reporting.
        cmd.stderr(Stdio::piped());

        // Spawn and wait with timeout.
        // We use child.wait() (takes &mut self) so we can still kill on timeout.
        let mut child = cmd.spawn()?;
        const TIMEOUT_SECS: u64 = 30;

        // Take stderr before waiting so we can read it after the process exits.
        let mut stderr_handle = child.stderr.take();

        let wait_result =
            tokio::time::timeout(Duration::from_secs(TIMEOUT_SECS), child.wait()).await;

        match wait_result {
            Ok(Ok(status)) => {
                // Read stderr from the taken handle.
                let mut stderr = String::new();
                if let Some(ref mut reader) = stderr_handle {
                    let _ = reader.read_to_string(&mut stderr).await;
                }

                let exit_code = status.code();
                let success = status.success();

                Ok(CompileResult {
                    success,
                    binary_path: if success {
                        Some(binary_path_str)
                    } else {
                        None
                    },
                    stderr,
                    exit_code,
                })
            }
            Ok(Err(e)) => Err(CompileError::Io(e)),
            Err(_elapsed) => {
                // Timeout reached — kill the child process and reap it.
                let _ = child.kill().await;
                let _ = child.wait().await;
                Err(CompileError::Timeout(TIMEOUT_SECS))
            }
        }
    }
}