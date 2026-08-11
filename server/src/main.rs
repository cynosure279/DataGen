//! DataGen Axum web server.
//!
//! Provides REST API endpoints backed by the DataGen engine:
//! - `GET /health` — health check
//! - `POST /api/generate` — generate test data (JSON body, 10s timeout)
//! - `POST /api/generate/stream` — SSE streaming generation (file-by-file progress)
//! - `POST /api/compile` — compile source code (multipart form)
//! - `POST /api/run` — run compiled binary (JSON body)
//! - `POST /api/generate-and-run` — full pipeline (generate + compile + run)
//! - `GET /api/compilers` — detect system compilers
//!
//! # Rate limiting
//! - 10 requests per 60 seconds per IP (tower-governor)
//! - Returns 429 Too Many Requests with Retry-After header
//!
//! # Concurrency control
//! - Semaphore with 4 permits for compile/execute operations
//! - Returns 503 Service Unavailable when busy
//!
//! # Sandbox (Linux only)
//! - setrlimit-based resource limits: 256M virtual memory, 30s CPU time
//! - 30s compile timeout, 5s run timeout
//! - Non-Linux: plain timeout fallback

use axum::{
    extract::{Multipart, State},
    http::{header::RETRY_AFTER, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use datagen_core::types::{GenResult, TestConfig};
use futures::stream::Stream;
use native::{
    compile::{CompileResult, CompileRunner},
    detect::{self, DetectionResult},
    execute::{ExecResult, ExecRunner},
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_stream::wrappers::ReceiverStream;
use tower_governor::{governor::GovernorConfigBuilder, GovernorError, GovernorLayer};

// ---------------------------------------------------------------------------
// Wrapper types — thin structs so they can be Arc-wrapped in AppState
// ---------------------------------------------------------------------------

/// Wraps [`datagen_core::orchestrator::generate`] for use as a stateful service.
pub struct Orchestrator;

impl Orchestrator {
    /// Generate test data from a [`TestConfig`].
    pub fn generate(
        &self,
        config: &TestConfig,
    ) -> Result<GenResult, datagen_core::config::ConfigError> {
        datagen_core::orchestrator::generate(config)
    }
}

/// Wraps [`native::detect::detect_compilers`] for use as a stateful service.
pub struct CompilerDetector;

impl CompilerDetector {
    /// Detect system compilers.
    pub fn detect(&self) -> DetectionResult {
        detect::detect_compilers()
    }
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for `POST /api/run`.
#[derive(Deserialize)]
pub struct RunRequest {
    pub binary_path: String,
    pub input: String,
    pub timeout: u64,
}

/// Request body for `POST /api/generate-and-run`.
#[derive(Deserialize)]
pub struct GenerateAndRunRequest {
    pub config: TestConfig,
    pub source: String,
    pub language: String,
}

/// Response body for `POST /api/generate-and-run`.
#[derive(Serialize)]
pub struct GenerateAndRunResponse {
    pub generation: GenResult,
    pub compilation: CompileResult,
    pub execution: Vec<ExecResult>,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Shared application state injected into all handlers via Axum's State extractor.
#[derive(Clone)]
pub struct AppState {
    pub generation_orchestrator: Arc<Orchestrator>,
    pub compiler_detector: Arc<CompilerDetector>,
    pub compile_runner: Arc<CompileRunner>,
    pub exec_runner: Arc<ExecRunner>,
    /// Semaphore limiting concurrent compile/execute to 4 permits.
    pub semaphore: Arc<Semaphore>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Unified API error type that renders as `{"error": "..."}` with appropriate status codes.
pub enum ApiError {
    BadRequest(String),
    Unprocessable(String),
    Internal(String),
    Timeout(String),
    /// 429 Too Many Requests — rate limit exceeded.
    RateLimited(String),
    /// 503 Service Unavailable — concurrency limit reached.
    Busy(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unprocessable(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Timeout(msg) => (StatusCode::REQUEST_TIMEOUT, msg),
            ApiError::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            ApiError::Busy(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
        };
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}

// ---------------------------------------------------------------------------
// Sandbox wrapper — Linux: setrlimit-based resource limits, non-Linux: timeout
// ---------------------------------------------------------------------------

/// Run a compiler inside a sandbox (Linux) or with plain timeout (other platforms).
///
/// On Linux, uses `std::process::Command` with `pre_exec` to set `setrlimit`
/// for virtual memory (256 MB) and CPU time (30 s) before exec.
/// On non-Linux, delegates to the native `CompileRunner`.
#[cfg(target_os = "linux")]
async fn sandboxed_compile(
    source: &str,
    language: &str,
    compiler_path: &str,
    args: &[String],
) -> Result<CompileResult, ApiError> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    // Python passthrough — no compilation needed.
    if language.eq_ignore_ascii_case("python") {
        return Ok(CompileResult {
            success: true,
            binary_path: None,
            stderr: String::new(),
            exit_code: Some(0),
        });
    }

    // Verify compiler exists.
    if which::which(compiler_path).is_err() {
        return Err(ApiError::BadRequest(format!(
            "Compiler not found: {compiler_path}"
        )));
    }

    // Determine source extension.
    let extension = match language {
        "c" => "c",
        "cpp" | "c++" => "cpp",
        _ => "cpp",
    };

    // Write source to a temp file (auto-cleaned on drop).
    let mut source_file = tempfile::NamedTempFile::with_suffix(format!(".{extension}"))
        .map_err(|e| ApiError::Internal(format!("Failed to create temp file: {e}")))?;
    source_file
        .write_all(source.as_bytes())
        .map_err(|e| ApiError::Internal(format!("Failed to write source: {e}")))?;
    let source_path = source_file.path().to_str().unwrap().to_string();

    // Derive output binary path.
    let binary_path = source_file.path().with_extension("");
    let binary_path_str = binary_path.to_str().unwrap().to_string();

    // Build compiler args: compiler -o output source [extra_args...]
    let mut cmd_args = vec!["-o".to_string(), binary_path_str.clone(), source_path];
    cmd_args.extend(args.iter().cloned());

    let compiler_path_owned = compiler_path.to_string();

    // Run in spawn_blocking because std::process::Command is synchronous.
    let (_stdout, stderr, exit_code) = tokio::task::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(&compiler_path_owned);
        cmd.args(&cmd_args);
        cmd.stderr(Stdio::piped());

        // Set resource limits before exec (Linux only).
        unsafe {
            cmd.pre_exec(|| {
                let mem_limit = libc::rlimit {
                    rlim_cur: 256 * 1024 * 1024,
                    rlim_max: 256 * 1024 * 1024,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &mem_limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                let cpu_limit = libc::rlimit {
                    rlim_cur: 30,
                    rlim_max: 30,
                };
                if libc::setrlimit(libc::RLIMIT_CPU, &cpu_limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(())
            });
        }

        let output = cmd.output()?;
        Ok::<_, std::io::Error>((
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.code(),
        ))
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Spawn blocking error: {e}")))?
    .map_err(|e| ApiError::Internal(format!("Compile IO error: {e}")))?;

    let success = exit_code == Some(0);

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

/// Fallback for non-Linux: use tokio::process::Command with timeout.
#[cfg(not(target_os = "linux"))]
async fn sandboxed_compile(
    source: &str,
    language: &str,
    compiler_path: &str,
    args: &[String],
) -> Result<CompileResult, ApiError> {
    let runner = CompileRunner;
    runner
        .compile(source, language, compiler_path, args)
        .await
        .map_err(|e| match e {
            native::compile::CompileError::Io(ioe) => {
                ApiError::Internal(format!("IO error: {ioe}"))
            }
            native::compile::CompileError::Timeout(s) => {
                ApiError::Timeout(format!("Compilation timed out after {s}s"))
            }
            native::compile::CompileError::CompilerNotFound(p) => {
                ApiError::BadRequest(format!("Compiler not found: {p}"))
            }
        })
}

/// Run a binary inside a sandbox (Linux) or with plain timeout (other platforms).
///
/// On Linux, uses `std::process::Command` with `pre_exec` to set `setrlimit`
/// for virtual memory (256 MB) and CPU time before exec.
/// Stdin is piped from the provided `input` string.
/// On non-Linux, delegates to the native `ExecRunner`.
#[cfg(target_os = "linux")]
async fn sandboxed_execute(
    binary_path: &str,
    input: &str,
    timeout_secs: u64,
) -> Result<ExecResult, ApiError> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    let is_python = binary_path.ends_with(".py");
    let binary_path_owned = binary_path.to_string();
    let input_owned = input.to_string();

    let (stdout, stderr, exit_code, timed_out) = tokio::task::spawn_blocking(move || {
        let mut cmd = if is_python {
            let mut c = std::process::Command::new("python3");
            c.arg(&binary_path_owned);
            c
        } else {
            std::process::Command::new(&binary_path_owned)
        };

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set resource limits before exec (Linux only).
        let ts = timeout_secs;
        unsafe {
            cmd.pre_exec(move || {
                let mem_limit = libc::rlimit {
                    rlim_cur: 256 * 1024 * 1024,
                    rlim_max: 256 * 1024 * 1024,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &mem_limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                let cpu_limit = libc::rlimit {
                    rlim_cur: ts,
                    rlim_max: ts,
                };
                if libc::setrlimit(libc::RLIMIT_CPU, &cpu_limit) != 0 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(())
            });
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return (
                    String::new(),
                    format!("Failed to spawn process: {e}"),
                    None,
                    false,
                );
            }
        };

        // Write input to stdin, then drop to signal EOF.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input_owned.as_bytes());
            // Drop closes stdin.
        }

        // Wait with timeout via a separate thread.
        let handle = std::thread::spawn(move || child.wait_with_output());
        match handle.join() {
            Ok(Ok(output)) => {
                let out = String::from_utf8_lossy(&output.stdout).to_string();
                let err = String::from_utf8_lossy(&output.stderr).to_string();
                (out, err, output.status.code(), false)
            }
            Ok(Err(e)) => {
                (String::new(), format!("IO error: {e}"), None, false)
            }
            Err(_) => {
                // Thread panicked — likely CPU limit killed the process.
                (String::new(), String::new(), None, true)
            }
        }
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Spawn blocking error: {e}")))?;

    Ok(ExecResult {
        stdout,
        stderr,
        exit_code,
        timed_out,
        killed: timed_out,
    })
}

/// Fallback for non-Linux: use tokio::process::Command with timeout.
#[cfg(not(target_os = "linux"))]
async fn sandboxed_execute(
    binary_path: &str,
    input: &str,
    timeout_secs: u64,
) -> Result<ExecResult, ApiError> {
    let runner = ExecRunner;
    runner
        .execute(binary_path, input, timeout_secs)
        .await
        .map_err(|e| ApiError::Internal(format!("Execution error: {e}")))
}

// ---------------------------------------------------------------------------
// Rate limiting error handler
// ---------------------------------------------------------------------------

/// Custom error handler for tower-governor that returns JSON with Retry-After.
fn rate_limit_error_handler(error: GovernorError) -> Response {
    match error {
        GovernorError::TooManyRequests { wait_time, headers } => {
            let body = serde_json::json!({
                "error": "Rate limit exceeded. Try again later.",
                "retry_after": wait_time,
            });
            let mut resp = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();
            if let Some(hdrs) = headers {
                resp.headers_mut().extend(hdrs);
            }
            // Ensure Retry-After header is present.
            if !resp.headers().contains_key(RETRY_AFTER) {
                resp.headers_mut().insert(
                    RETRY_AFTER,
                    wait_time.to_string().parse().unwrap(),
                );
            }
            resp
        }
        GovernorError::UnableToExtractKey => {
            let body = serde_json::json!({"error": "Could not determine client IP"});
            (StatusCode::BAD_REQUEST, Json(body)).into_response()
        }
        GovernorError::Other { code, msg, headers } => {
            let body = serde_json::json!({
                "error": msg.unwrap_or_else(|| "Rate limiting error".to_string()),
            });
            let mut resp = (code, Json(body)).into_response();
            if let Some(hdrs) = headers {
                resp.headers_mut().extend(hdrs);
            }
            resp
        }
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /health` — returns 200 OK with a plain-text message.
async fn health_handler() -> impl IntoResponse {
    "DataGen Server running"
}

/// `POST /api/generate` — generate test data from a JSON TestConfig.
/// 10s timeout. Returns JSON GenResult.
async fn generate_handler(
    State(state): State<AppState>,
    Json(config): Json<TestConfig>,
) -> Result<Json<GenResult>, ApiError> {
    let orch = state.generation_orchestrator.clone();

    match tokio::time::timeout(
        Duration::from_secs(10),
        tokio::task::spawn_blocking(move || orch.generate(&config)),
    )
    .await
    {
        Ok(Ok(Ok(result))) => Ok(Json(result)),
        Ok(Ok(Err(e))) => Err(ApiError::Unprocessable(e.to_string())),
        Ok(Err(join_err)) => {
            Err(ApiError::Internal(format!("Generation task failed: {join_err}")))
        }
        Err(_) => Err(ApiError::Timeout(
            "Generation timed out after 10s".into(),
        )),
    }
}

/// `POST /api/compile` — compile source code from multipart form.
/// Fields: source, language, compiler, args (repeatable for multiple args).
async fn compile_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<CompileResult>, ApiError> {
    let mut source = None;
    let mut language = None;
    let mut compiler = None;
    let mut args = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read multipart field: {e}")))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "source" => {
                source = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read source: {e}")))?,
                )
            }
            "language" => {
                language = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read language: {e}")))?,
                )
            }
            "compiler" => {
                compiler = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read compiler: {e}")))?,
                )
            }
            "args" => {
                args.push(
                    field
                        .text()
                        .await
                        .map_err(|e| ApiError::BadRequest(format!("Failed to read args: {e}")))?,
                );
            }
            _ => {}
        }
    }

    let source = source.ok_or_else(|| ApiError::BadRequest("Missing field: source".into()))?;
    let language =
        language.ok_or_else(|| ApiError::BadRequest("Missing field: language".into()))?;
    let compiler =
        compiler.ok_or_else(|| ApiError::BadRequest("Missing field: compiler".into()))?;

    // Acquire concurrency permit (release on drop).
    let _permit = state
        .semaphore
        .try_acquire()
        .map_err(|_| ApiError::Busy("Server busy, too many concurrent compilations. Try again later.".into()))?;

    let result = sandboxed_compile(&source, &language, &compiler, &args).await?;

    Ok(Json(result))
}

/// `POST /api/run` — run a compiled binary with piped input.
async fn run_handler(
    State(state): State<AppState>,
    Json(req): Json<RunRequest>,
) -> Result<Json<ExecResult>, ApiError> {
    // Acquire concurrency permit (release on drop).
    let _permit = state
        .semaphore
        .try_acquire()
        .map_err(|_| ApiError::Busy("Server busy, too many concurrent executions. Try again later.".into()))?;

    let result = sandboxed_execute(&req.binary_path, &req.input, req.timeout).await?;

    Ok(Json(result))
}

/// `POST /api/generate-and-run` — full pipeline: generate test data, compile source,
/// then run the binary against each generated file.
async fn generate_and_run_handler(
    State(state): State<AppState>,
    Json(req): Json<GenerateAndRunRequest>,
) -> Result<Json<GenerateAndRunResponse>, ApiError> {
    // Step 1: Generate test data (no semaphore needed — CPU-light orchestration).
    let gen_result = state
        .generation_orchestrator
        .generate(&req.config)
        .map_err(|e| ApiError::Unprocessable(format!("Generation error: {e}")))?;

    // Step 2: Compile source — auto-select compiler based on language.
    let (compiler_path, args): (&str, &[String]) = match req.language.to_lowercase().as_str() {
        "c" => ("gcc", &[]),
        "cpp" | "c++" => ("g++", &[]),
        "python" => ("python3", &[]),
        lang => return Err(ApiError::BadRequest(format!("Unsupported language: {lang}"))),
    };

    // Acquire compile permit.
    let _compile_permit = state
        .semaphore
        .try_acquire()
        .map_err(|_| ApiError::Busy("Server busy, too many concurrent compilations. Try again later.".into()))?;

    let compile_result = sandboxed_compile(&req.source, &req.language, compiler_path, args).await?;

    // Release compile permit (drop _compile_permit).
    drop(_compile_permit);

    // Step 3: Run binary against each generated file.
    let mut exec_results = Vec::new();

    if let Some(ref binary_path) = compile_result.binary_path {
        // Compiled binary exists — run it against each file's content as input.
        for file in &gen_result.files {
            // Acquire execute permit per run.
            let _exec_permit = state
                .semaphore
                .try_acquire()
                .map_err(|_| {
                    ApiError::Busy(
                        "Server busy, too many concurrent executions. Try again later.".into(),
                    )
                })?;

            let exec_result =
                sandboxed_execute(binary_path, &file.content, 5).await?;
            exec_results.push(exec_result);
        }
    } else if req.language.eq_ignore_ascii_case("python") {
        // Python: write source to a temp file and run with python3.
        let dir = tempfile::TempDir::new()
            .map_err(|e| ApiError::Internal(format!("Failed to create temp dir: {e}")))?;
        let script_path = dir.path().join("script.py");
        std::fs::write(&script_path, &req.source)
            .map_err(|e| ApiError::Internal(format!("Failed to write script: {e}")))?;

        for file in &gen_result.files {
            // Acquire execute permit per run.
            let _exec_permit = state
                .semaphore
                .try_acquire()
                .map_err(|_| {
                    ApiError::Busy(
                        "Server busy, too many concurrent executions. Try again later.".into(),
                    )
                })?;

            let exec_result = sandboxed_execute(
                script_path.to_str().unwrap(),
                &file.content,
                5,
            )
            .await?;
            exec_results.push(exec_result);
        }
        // `dir` is dropped here — temp files cleaned up automatically.
    }

    Ok(Json(GenerateAndRunResponse {
        generation: gen_result,
        compilation: compile_result,
        execution: exec_results,
    }))
}

/// `GET /api/compilers` — detect available system compilers.
async fn compilers_handler(State(state): State<AppState>) -> Json<DetectionResult> {
    Json(state.compiler_detector.detect())
}

/// `POST /api/generate/stream` — SSE endpoint that streams file-by-file progress events.
///
/// Events:
/// - `progress`: `{"file": "...", "index": N, "total": M}`
/// - `complete`: full `GenResult` JSON
/// - `error`: `{"error": "..."}`
async fn generate_stream_handler(
    State(state): State<AppState>,
    Json(config): Json<TestConfig>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(32);
    let orch = state.generation_orchestrator.clone();

    tokio::spawn(async move {
        match orch.generate(&config) {
            Ok(result) => {
                let total = result.files.len() as u32;
                for (i, file) in result.files.iter().enumerate() {
                    let progress = serde_json::json!({
                        "file": file.filename,
                        "index": i as u32 + 1,
                        "total": total,
                    });
                    let event = Event::default()
                        .event("progress")
                        .data(serde_json::to_string(&progress).unwrap_or_default());
                    if tx.send(Ok(event)).await.is_err() {
                        break;
                    }
                }
                let complete_event = Event::default()
                    .event("complete")
                    .data(serde_json::to_string(&result).unwrap_or_default());
                let _ = tx.send(Ok(complete_event)).await;
            }
            Err(e) => {
                let error_event = Event::default()
                    .event("error")
                    .data(serde_json::json!({"error": e.to_string()}).to_string());
                let _ = tx.send(Ok(error_event)).await;
            }
        }
    });

    Sse::new(ReceiverStream::new(rx))
}

// ---------------------------------------------------------------------------
// Router setup
// ---------------------------------------------------------------------------

fn app(state: AppState) -> Router {
    // Rate limiting: 10 requests per 60 seconds per IP.
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(10)
            .period(Duration::from_secs(6))
            .use_headers()
            .finish()
            .unwrap(),
    );

    // Background cleanup of stale rate limiting entries.
    let governor_limiter = governor_config.limiter().clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(120));
        loop {
            interval.tick().await;
            governor_limiter.retain_recent();
        }
    });

    Router::new()
        .route("/health", get(health_handler))
        .route("/api/generate", post(generate_handler))
        .route("/api/generate/stream", post(generate_stream_handler))
        .route("/api/compile", post(compile_handler))
        .route("/api/run", post(run_handler))
        .route("/api/generate-and-run", post(generate_and_run_handler))
        .route("/api/compilers", get(compilers_handler))
        .layer(
            GovernorLayer::new(governor_config)
                .error_handler(|e: GovernorError| rate_limit_error_handler(e)),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::limit::RequestBodyLimitLayer::new(50 * 1024 * 1024))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------

/// Waits for SIGINT (Ctrl+C) or SIGTERM, then returns to trigger graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "datagen_server=info,tower_http=info".into()),
        )
        .init();

    let state = AppState {
        generation_orchestrator: Arc::new(Orchestrator),
        compiler_detector: Arc::new(CompilerDetector),
        compile_runner: Arc::new(CompileRunner),
        exec_runner: Arc::new(ExecRunner),
        semaphore: Arc::new(Semaphore::new(4)),
    };

    let app = app(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind TCP listener");

    tracing::info!("DataGen Server listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server exited with error");
}