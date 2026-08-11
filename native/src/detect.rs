//! Compiler detection utilities.
//!
//! Detects system compilers (gcc, g++, clang, clang++, python3)
//! using the `which` crate on Linux, and provides install guidance.

use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;

/// Information about a detected compiler.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompilerInfo {
    pub name: String,
    pub path: String,
    pub version: String,
}

/// Result of compiler detection.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DetectionResult {
    pub found: Vec<CompilerInfo>,
    pub missing: Vec<String>,
}

/// Compilers we attempt to detect.
const COMPILERS: &[&str] = &["gcc", "g++", "clang", "clang++", "python3"];

/// Detect system compilers using `which` and version probes.
pub fn detect_compilers() -> DetectionResult {
    let mut found = Vec::new();
    let mut missing = Vec::new();

    for name in COMPILERS {
        match which::which(name) {
            Ok(path) => {
                let version = extract_version(name, &path);
                found.push(CompilerInfo {
                    name: name.to_string(),
                    path: path.to_string_lossy().to_string(),
                    version,
                });
            }
            Err(_) => {
                missing.push(name.to_string());
            }
        }
    }

    DetectionResult { found, missing }
}

/// Extract the first line of `<compiler> --version`.
fn extract_version(name: &str, path: &std::path::Path) -> String {
    Command::new(path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.lines().next().map(|s| s.trim().to_string())
            } else {
                // Some compilers (e.g. python3) print version to stderr
                let stderr = String::from_utf8_lossy(&output.stderr);
                stderr.lines().next().map(|s| s.trim().to_string())
            }
        })
        .unwrap_or_else(|| {
            // Fallback: try `name --version` directly (for cross-platform)
            Command::new(name)
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| {
                    let s = String::from_utf8_lossy(&o.stdout);
                    s.lines().next().map(|l| l.trim().to_string())
                })
                .unwrap_or_default()
        })
}

/// Return install guidance for each compiler, keyed by compiler name.
pub fn install_guidance() -> HashMap<String, String> {
    let mut map = HashMap::new();

    #[cfg(target_os = "linux")]
    {
        map.insert("gcc".into(), "apt install gcc".into());
        map.insert("g++".into(), "apt install g++".into());
        map.insert("clang".into(), "apt install clang".into());
        map.insert("clang++".into(), "apt install clang++".into());
        map.insert("python3".into(), "apt install python3".into());
    }

    #[cfg(target_os = "macos")]
    {
        map.insert("gcc".into(), "brew install gcc".into());
        map.insert("g++".into(), "brew install gcc".into());
        map.insert("clang".into(), "brew install llvm".into());
        map.insert("clang++".into(), "brew install llvm".into());
        map.insert("python3".into(), "brew install python".into());
    }

    #[cfg(target_os = "windows")]
    {
        map.insert("gcc".into(), "Install MinGW-w64 from https://winlibs.com/".into());
        map.insert("g++".into(), "Install MinGW-w64 from https://winlibs.com/".into());
        map.insert("clang".into(), "Install LLVM from https://releases.llvm.org/".into());
        map.insert("clang++".into(), "Install LLVM from https://releases.llvm.org/".into());
        map.insert("python3".into(), "Install Python from https://python.org/".into());
    }

    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_at_least_one_compiler_found() {
        let result = detect_compilers();
        // On a typical Linux CI / dev machine at least one compiler should exist
        assert!(
            !result.found.is_empty(),
            "Expected at least one compiler to be found, got none. missing: {:?}",
            result.missing
        );
    }

    #[test]
    fn test_missing_compiler_appears_in_missing_list() {
        let result = detect_compilers();
        // If a known compiler is missing, it should be in the missing list
        // We can't guarantee any specific compiler is missing, but we can
        // verify the invariant: every compiler we checked is either found or missing
        let all_names: Vec<String> = COMPILERS.iter().map(|s| s.to_string()).collect();
        let found_names: Vec<String> = result.found.iter().map(|c| c.name.clone()).collect();
        let missing_names = &result.missing;

        for name in &all_names {
            assert!(
                found_names.contains(name) || missing_names.contains(name),
                "Compiler '{}' is neither found nor missing — detection incomplete",
                name
            );
        }
    }

    #[test]
    fn test_found_compilers_have_non_empty_version() {
        let result = detect_compilers();
        for compiler in &result.found {
            assert!(
                !compiler.version.is_empty(),
                "Compiler '{}' at '{}' has empty version string",
                compiler.name,
                compiler.path
            );
        }
    }

    #[test]
    fn test_found_compilers_have_non_empty_path() {
        let result = detect_compilers();
        for compiler in &result.found {
            assert!(
                !compiler.path.is_empty(),
                "Compiler '{}' has empty path",
                compiler.name
            );
        }
    }

    #[test]
    fn test_install_guidance_has_all_compilers() {
        let guidance = install_guidance();
        for name in COMPILERS {
            assert!(
                guidance.contains_key(*name),
                "install_guidance missing entry for '{}'",
                name
            );
        }
    }

    #[test]
    fn test_install_guidance_instructions_non_empty() {
        let guidance = install_guidance();
        for (name, instruction) in &guidance {
            assert!(
                !instruction.is_empty(),
                "install_guidance for '{}' is empty",
                name
            );
        }
    }

    #[test]
    fn test_detect_result_debug_and_clone() {
        let result = detect_compilers();
        // Verify Debug + Clone bounds compile and work
        let _cloned = result.clone();
        let _debug = format!("{:?}", result);
    }

    #[test]
    fn test_compiler_info_debug_and_clone() {
        let result = detect_compilers();
        if let Some(compiler) = result.found.first() {
            let _cloned = compiler.clone();
            let _debug = format!("{:?}", compiler);
        }
    }
}