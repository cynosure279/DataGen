//! Tauri command bridge — thin wrappers around core engine + compile/execute runners.
//!
//! All commands return `Result<String, String>` where the `Ok` value is a JSON
//! string and the `Err` value is a human-readable error message.

use serde::{Deserialize, Serialize};
use serde_json;

use crate::compile::{CompileResult, CompileRunner};
use crate::detect;
use crate::execute::{ExecResult, ExecRunner};

use datagen_core::orchestrator;
use datagen_core::types::{GenResult, TestConfig};

// ---------------------------------------------------------------------------
// Command 1: detect_compilers
// ---------------------------------------------------------------------------

/// Detect system compilers (gcc, g++, clang, clang++, python3).
///
/// Returns JSON `DetectionResult { found: [...], missing: [...] }`.
#[tauri::command]
pub fn detect_compilers() -> Result<String, String> {
    let result = detect::detect_compilers();
    serde_json::to_string(&result).map_err(|e| format!("Serialization error: {}", e))
}

// ---------------------------------------------------------------------------
// Command 2: generate_data
// ---------------------------------------------------------------------------

/// Generate test data from a JSON-serialized `TestConfig`.
///
/// Returns JSON `GenResult { files: [...], metadata: {...} }`.
#[tauri::command]
pub fn generate_data(config_json: String) -> Result<String, String> {
    let config: TestConfig =
        serde_json::from_str(&config_json).map_err(|e| format!("Invalid config JSON: {}", e))?;

    let result = orchestrator::generate(&config).map_err(|e| format!("Generation failed: {}", e))?;

    serde_json::to_string(&result).map_err(|e| format!("Serialization error: {}", e))
}

// ---------------------------------------------------------------------------
// Command 3: compile_code
// ---------------------------------------------------------------------------

/// Compile source code using a system compiler.
///
/// - `source`: source code text
/// - `language`: `"c"`, `"cpp"`, or `"python"` (case-insensitive)
/// - `compiler`: compiler binary name/path (e.g. `"g++"`)
/// - `args`: extra compiler flags (e.g. `["-O2", "-std=c++17"]`)
///
/// Returns JSON `CompileResult { success, binary_path, stderr, exit_code }`.
#[tauri::command]
pub async fn compile_code(
    source: String,
    language: String,
    compiler: String,
    args: Vec<String>,
) -> Result<String, String> {
    let runner = CompileRunner;
    let result = runner
        .compile(&source, &language, &compiler, &args)
        .await
        .map_err(|e| format!("Compilation error: {}", e))?;

    serde_json::to_string(&result).map_err(|e| format!("Serialization error: {}", e))
}

// ---------------------------------------------------------------------------
// Command 4: run_binary
// ---------------------------------------------------------------------------

/// Execute a compiled binary (or Python script) with piped input.
///
/// - `binary_path`: path to the binary or `.py` script
/// - `input`: stdin content
/// - `timeout`: max execution time in seconds
///
/// Returns JSON `ExecResult { stdout, stderr, exit_code, timed_out, killed }`.
#[tauri::command]
pub async fn run_binary(
    binary_path: String,
    input: String,
    timeout: u64,
) -> Result<String, String> {
    let runner = ExecRunner;
    let result = runner
        .execute(&binary_path, &input, timeout)
        .await
        .map_err(|e| format!("Execution error: {}", e))?;

    serde_json::to_string(&result).map_err(|e| format!("Serialization error: {}", e))
}

// ---------------------------------------------------------------------------
// Command 5: generate_and_run
// ---------------------------------------------------------------------------

/// Full flow: generate test data → compile source → run binary with generated input.
///
/// - `config_json`: JSON-serialized `TestConfig`
/// - `source`: source code text
/// - `language`: `"c"`, `"cpp"`, or `"python"`
///
/// Returns JSON `{ generation: GenResult, compilation: CompileResult, execution: ExecResult }`.
#[tauri::command]
pub async fn generate_and_run(
    config_json: String,
    source: String,
    language: String,
) -> Result<String, String> {
    // 1. Parse config
    let config: TestConfig =
        serde_json::from_str(&config_json).map_err(|e| format!("Invalid config JSON: {}", e))?;

    // 2. Generate test data
    let gen_result =
        orchestrator::generate(&config).map_err(|e| format!("Generation failed: {}", e))?;

    // 3. Use first generated file's content as program input
    let input = gen_result
        .files
        .first()
        .map(|f| f.content.clone())
        .unwrap_or_default();

    // 4. Compile source (default to g++, empty args)
    let compiler = "g++";
    let compile_runner = CompileRunner;
    let compile_result = compile_runner
        .compile(&source, &language, compiler, &[])
        .await
        .map_err(|e| format!("Compilation error: {}", e))?;

    if !compile_result.success {
        return Err(format!(
            "Compilation failed: {}",
            compile_result.stderr
        ));
    }

    // 5. Run the compiled binary with generated input
    let exec_runner = ExecRunner;
    let binary_path = compile_result
        .binary_path
        .as_ref()
        .ok_or_else(|| "Compilation succeeded but no binary path returned".to_string())?;

    let exec_result = exec_runner
        .execute(binary_path, &input, 30)
        .await
        .map_err(|e| format!("Execution error: {}", e))?;

    // 6. Serialize combined output
    #[derive(Serialize)]
    struct GenerateAndRunOutput {
        generation: GenResult,
        compilation: CompileResult,
        execution: ExecResult,
    }

    let output = GenerateAndRunOutput {
        generation: gen_result,
        compilation: compile_result,
        execution: exec_result,
    };

    serde_json::to_string(&output).map_err(|e| format!("Serialization error: {}", e))
}

// ---------------------------------------------------------------------------
// Command 6: save_files
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct SaveFile { filename: String, content: String }

/// Save generated/compiled files to a directory on disk.
#[tauri::command]
pub fn save_files(dir: String, files_json: String) -> Result<String, String> {
    let files: Vec<SaveFile> = serde_json::from_str(&files_json)
        .map_err(|e| format!("Invalid files JSON: {}", e))?;
    let dir_path = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir_path).map_err(|e| format!("Cannot create dir: {}", e))?;
    let mut saved = Vec::new();
    for f in &files {
        let path = dir_path.join(&f.filename);
        std::fs::write(&path, &f.content).map_err(|e| format!("Write {}: {}", f.filename, e))?;
        saved.push(f.filename.clone());
    }
    Ok(serde_json::to_string(&saved).unwrap_or_default())
}