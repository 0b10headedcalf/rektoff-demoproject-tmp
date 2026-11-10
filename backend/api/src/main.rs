//! HTTP API for the practice platform.
//!
//! Accepts a player's source file, compiles it to SBF, grades the result with
//! the grader, and reports progress over Server-Sent Events.
//!
//! Jobs run one at a time through a bounded queue. All state is in memory.

use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path as UrlPath, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::{mpsc, watch};
use tokio_stream::wrappers::WatchStream;
use tokio_stream::{Stream, StreamExt};

const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_COMPILE_ERROR_BYTES: usize = 8 * 1024;
const QUEUE_CAPACITY: usize = 20;
const BUILD_TIMEOUT: Duration = Duration::from_secs(60);
const TEST_TIMEOUT: Duration = Duration::from_secs(20);
const JOB_RETENTION: Duration = Duration::from_secs(600);

struct Level {
    description: String,
    source: String,
    /// Private copy of the program crate where submissions are compiled.
    work_dir: PathBuf,
}

struct Job {
    id: String,
    level: String,
    source: String,
    enqueued: Instant,
    state: watch::Sender<JobState>,
}

#[derive(Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Stage {
    #[default]
    Queued,
    Building,
    Testing,
    Done,
}

#[derive(Clone, Default, Serialize)]
struct CheckOutcome {
    name: String,
    passed: bool,
}

/// Everything the player is allowed to see about a job.
#[derive(Clone, Default, Serialize)]
struct JobState {
    stage: Stage,
    passed: Option<bool>,
    checks: Vec<CheckOutcome>,
    compile_error: Option<String>,
    error: Option<String>,
    queue_ms: Option<u64>,
    build_ms: Option<u64>,
    test_ms: Option<u64>,
}

struct App {
    levels: HashMap<String, Level>,
    jobs: Mutex<HashMap<String, (Instant, watch::Receiver<JobState>)>>,
    queue: mpsc::Sender<Job>,
    grader: PathBuf,
}

#[derive(Serialize)]
struct LevelResponse<'a> {
    id: &'a str,
    description: &'a str,
    source: &'a str,
}

#[derive(Deserialize)]
struct SubmitRequest {
    level: String,
    source: String,
}

#[derive(Serialize)]
struct SubmitResponse {
    id: String,
}

type ApiError = (StatusCode, &'static str);

#[derive(Serialize)]
struct LevelSummary<'a> {
    id: &'a str,
    title: &'a str,
}

/// Every level in play order (ids start with their number), for the level picker.
async fn list_levels(State(app): State<Arc<App>>) -> Json<serde_json::Value> {
    let mut levels: Vec<LevelSummary> = app
        .levels
        .iter()
        .map(|(id, level)| {
            let heading = level.description.lines().find_map(|line| line.strip_prefix("# "));
            LevelSummary { id, title: heading.unwrap_or(id) }
        })
        .collect();
    levels.sort_by_key(|level| level.id);
    Json(serde_json::to_value(levels).expect("levels serialize"))
}

async fn get_level(
    State(app): State<Arc<App>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let level = app.levels.get(&id).ok_or((StatusCode::NOT_FOUND, "unknown level"))?;
    let body = LevelResponse { id: &id, description: &level.description, source: &level.source };
    Ok(Json(serde_json::to_value(body).expect("level serializes")))
}

async fn submit(
    State(app): State<Arc<App>>,
    Json(request): Json<SubmitRequest>,
) -> Result<Json<SubmitResponse>, ApiError> {
    if !app.levels.contains_key(&request.level) {
        return Err((StatusCode::NOT_FOUND, "unknown level"));
    }
    if request.source.len() > MAX_SOURCE_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "source too large"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let (state, receiver) = watch::channel(JobState::default());
    let job = Job {
        id: id.clone(),
        level: request.level,
        source: request.source,
        enqueued: Instant::now(),
        state,
    };
    app.queue
        .try_send(job)
        .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "queue is full, try again shortly"))?;

    let mut jobs = app.jobs.lock().expect("jobs lock");
    jobs.retain(|_, (created, _)| created.elapsed() < JOB_RETENTION);
    jobs.insert(id.clone(), (Instant::now(), receiver));
    Ok(Json(SubmitResponse { id }))
}

fn job_receiver(app: &App, id: &str) -> Result<watch::Receiver<JobState>, ApiError> {
    let jobs = app.jobs.lock().expect("jobs lock");
    jobs.get(id).map(|(_, rx)| rx.clone()).ok_or((StatusCode::NOT_FOUND, "unknown job"))
}

async fn get_job(
    State(app): State<Arc<App>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<JobState>, ApiError> {
    let receiver = job_receiver(&app, &id)?;
    let state = receiver.borrow().clone();
    Ok(Json(state))
}

/// Streams every state change. A watch channel always yields its current
/// value first, so a client that connects after the job finished still gets
/// the final result.
async fn job_events(
    State(app): State<Arc<App>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let receiver = job_receiver(&app, &id)?;
    let mut finished = false;
    let stream = WatchStream::new(receiver)
        .take_while(move |state| {
            let keep = !finished;
            finished = state.stage == Stage::Done;
            keep
        })
        .map(|state| {
            let name = if state.stage == Stage::Done { "done" } else { "status" };
            let data = serde_json::to_string(&state).expect("state serializes");
            Ok(Event::default().event(name).data(data))
        });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(5))))
}

/// A command that sees only the environment it needs, so `env!()` in a
/// submission cannot read anything else from the server's environment.
fn sandboxed_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command.env_clear().kill_on_drop(true).stdin(Stdio::null());
    for key in ["PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "RUSTUP_TOOLCHAIN", "CARGO_NET_OFFLINE"] {
        if let Ok(value) = env::var(key) {
            command.env(key, value);
        }
    }
    command
}

struct BuildFailure(String);

/// Compiles the program in `work_dir` and returns the path of the fresh `.so`.
async fn build(work_dir: &Path, source: &str) -> Result<PathBuf, BuildFailure> {
    let deploy_dir = work_dir.join("target/deploy");
    let io = |e: std::io::Error| BuildFailure(format!("server error: {e}"));

    // Remove old artifacts first so a failed build can never be graded as the
    // previous player's program.
    if let Ok(mut entries) = tokio::fs::read_dir(&deploy_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().extension().is_some_and(|ext| ext == "so") {
                tokio::fs::remove_file(entry.path()).await.map_err(io)?;
            }
        }
    }
    tokio::fs::write(work_dir.join("src/lib.rs"), source).await.map_err(io)?;

    let run = sandboxed_command("cargo-build-sbf")
        .current_dir(work_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let output = match tokio::time::timeout(BUILD_TIMEOUT, run).await {
        Err(_) => return Err(BuildFailure("build timed out".into())),
        Ok(result) => result.map_err(io)?,
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let start = stderr.len().saturating_sub(MAX_COMPILE_ERROR_BYTES);
        let start = (start..=stderr.len()).find(|&i| stderr.is_char_boundary(i)).unwrap_or(0);
        return Err(BuildFailure(stderr[start..].to_string()));
    }

    let mut entries = tokio::fs::read_dir(&deploy_dir).await.map_err(io)?;
    while let Some(entry) = entries.next_entry().await.map_err(io)? {
        if entry.path().extension().is_some_and(|ext| ext == "so") {
            return Ok(entry.path());
        }
    }
    Err(BuildFailure("build produced no program".into()))
}

#[derive(Deserialize)]
struct GraderCheck {
    name: String,
    passed: bool,
    detail: Option<String>,
}

#[derive(Deserialize)]
struct GraderReport {
    passed: bool,
    checks: Vec<GraderCheck>,
}

async fn grade(grader: &Path, level: &str, program: &Path) -> Result<GraderReport, String> {
    let run = sandboxed_command(&grader.to_string_lossy())
        .arg(level)
        .arg(program)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let output = tokio::time::timeout(TEST_TIMEOUT, run)
        .await
        .map_err(|_| "tests timed out".to_string())?
        .map_err(|e| format!("could not start grader: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().last().unwrap_or_default();
    serde_json::from_str(line).map_err(|e| format!("bad grader output: {e}"))
}

async fn run_job(app: &App, job: Job) {
    let level = &app.levels[&job.level];
    let queue_ms = job.enqueued.elapsed().as_millis() as u64;
    job.state.send_modify(|s| {
        s.stage = Stage::Building;
        s.queue_ms = Some(queue_ms);
    });

    let started = Instant::now();
    let built = build(&level.work_dir, &job.source).await;
    let build_ms = started.elapsed().as_millis() as u64;
    let program = match built {
        Ok(program) => program,
        Err(BuildFailure(message)) => {
            job.state.send_modify(|s| {
                s.stage = Stage::Done;
                s.build_ms = Some(build_ms);
                s.passed = Some(false);
                s.compile_error = Some(message);
            });
            println!("job {} level={} queue={queue_ms}ms build={build_ms}ms result=build-failed", job.id, job.level);
            return;
        }
    };
    job.state.send_modify(|s| {
        s.stage = Stage::Testing;
        s.build_ms = Some(build_ms);
    });

    let started = Instant::now();
    let graded = grade(&app.grader, &job.level, &program).await;
    let test_ms = started.elapsed().as_millis() as u64;
    match graded {
        Ok(report) => {
            for check in report.checks.iter().filter(|c| !c.passed) {
                // `detail` describes hidden test behaviour: log it, never send it.
                println!("job {} check failed: {:?}: {:?}", job.id, check.name, check.detail);
            }
            println!(
                "job {} level={} queue={queue_ms}ms build={build_ms}ms test={test_ms}ms passed={}",
                job.id, job.level, report.passed
            );
            job.state.send_modify(|s| {
                s.stage = Stage::Done;
                s.test_ms = Some(test_ms);
                s.passed = Some(report.passed);
                s.checks = report
                    .checks
                    .into_iter()
                    .map(|c| CheckOutcome { name: c.name, passed: c.passed })
                    .collect();
            });
        }
        Err(message) => {
            eprintln!("job {} grading error: {message}", job.id);
            job.state.send_modify(|s| {
                s.stage = Stage::Done;
                s.test_ms = Some(test_ms);
                s.passed = Some(false);
                s.error = Some("the grader failed, please try again".into());
            });
        }
    }
}

/// Copies each level's program crate into the work directory and compiles the
/// original source once, so the first player does not pay for a cold build.
async fn load_levels(levels_dir: &Path, work_root: &Path) -> HashMap<String, Level> {
    let mut levels = HashMap::new();
    let mut entries = tokio::fs::read_dir(levels_dir).await.expect("LEVELS_DIR is readable");
    while let Some(entry) = entries.next_entry().await.expect("read LEVELS_DIR") {
        let program_dir = entry.path().join("program");
        let Ok(source) = tokio::fs::read_to_string(program_dir.join("src/lib.rs")).await else {
            continue;
        };
        let id = entry.file_name().to_string_lossy().to_string();
        let description = tokio::fs::read_to_string(entry.path().join("description.md"))
            .await
            .unwrap_or_default();

        let level_work = work_root.join(&id);
        let work_dir = level_work.join("program");
        if !work_dir.exists() {
            tokio::fs::create_dir_all(&level_work).await.expect("create work dir");
            let copied = Command::new("cp").arg("-a").arg(&program_dir).arg(&level_work).status().await;
            assert!(copied.is_ok_and(|s| s.success()), "could not copy {id} into the work dir");
        }

        println!("warming {id} (a cold build can take a minute)...");
        let started = Instant::now();
        match tokio::time::timeout(Duration::from_secs(600), warm(&work_dir, &source)).await {
            Ok(Ok(())) => println!("warmed {id} in {:.1}s", started.elapsed().as_secs_f32()),
            Ok(Err(message)) => panic!("level {id} does not build:\n{message}"),
            Err(_) => panic!("level {id} warm-up timed out"),
        }
        levels.insert(id, Level { description, source, work_dir });
    }
    levels
}

async fn warm(work_dir: &Path, source: &str) -> Result<(), String> {
    tokio::fs::write(work_dir.join("src/lib.rs"), source).await.map_err(|e| e.to_string())?;
    let output = sandboxed_command("cargo-build-sbf")
        .current_dir(work_dir)
        .output()
        .await
        .map_err(|e| format!("could not run cargo-build-sbf (is it on PATH?): {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn env_path(key: &str, default: &str) -> PathBuf {
    PathBuf::from(env::var(key).unwrap_or_else(|_| default.to_string()))
}

#[tokio::main]
async fn main() {
    let levels_dir = env_path("LEVELS_DIR", "levels");
    let work_root = env_path("WORK_DIR", ".work");
    let grader = env_path("GRADER_BIN", "target/release/grader");
    let port = env::var("PORT").unwrap_or_else(|_| "8787".to_string());

    let grader = grader.canonicalize().expect("GRADER_BIN exists (cargo build --release -p grader)");
    tokio::fs::create_dir_all(&work_root).await.expect("create WORK_DIR");
    let work_root = work_root.canonicalize().expect("WORK_DIR resolves");
    let levels = load_levels(&levels_dir, &work_root).await;
    assert!(!levels.is_empty(), "no levels found in {}", levels_dir.display());

    let (queue, mut jobs) = mpsc::channel::<Job>(QUEUE_CAPACITY);
    let app = Arc::new(App { levels, jobs: Mutex::new(HashMap::new()), queue, grader });

    let worker_app = app.clone();
    tokio::spawn(async move {
        while let Some(job) = jobs.recv().await {
            run_job(&worker_app, job).await;
        }
    });

    let router = Router::new()
        .route("/api/levels", get(list_levels))
        .route("/api/levels/{id}", get(get_level))
        .route("/api/submit", post(submit))
        .route("/api/submit/{id}", get(get_job))
        .route("/api/submit/{id}/events", get(job_events))
        .with_state(app);

    let address = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await.expect("bind port");
    println!("api listening on http://{address}");
    axum::serve(listener, router).await.expect("server runs");
}
