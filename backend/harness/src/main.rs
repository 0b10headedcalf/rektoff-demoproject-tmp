//! Trusted grader. Loads a compiled program and runs a level's checks against it.
//!
//! Usage: harness <level> <path/to/program.so>
//! Prints one JSON object on stdout. `detail` is for logs only: the API must
//! strip it before anything reaches the player.

use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use serde::Serialize;

#[path = "../../levels/level-01-missing-signer/tests-hidden/checks.rs"]
mod level_01;

pub struct Check {
    pub name: &'static str,
    pub run: fn(&Path) -> Result<(), String>,
}

#[derive(Serialize)]
struct CheckResult {
    name: &'static str,
    passed: bool,
    detail: Option<String>,
    millis: u128,
}

#[derive(Serialize)]
struct Report {
    level: String,
    passed: bool,
    checks: Vec<CheckResult>,
}

fn checks_for(level: &str) -> Option<&'static [Check]> {
    match level {
        "level-01-missing-signer" => Some(level_01::CHECKS),
        _ => None,
    }
}

fn run_check(check: &Check, program: &Path) -> CheckResult {
    let started = Instant::now();
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| (check.run)(program)))
        .unwrap_or_else(|_| Err("check panicked".to_string()));
    CheckResult {
        name: check.name,
        passed: outcome.is_ok(),
        detail: outcome.err(),
        millis: started.elapsed().as_millis(),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [level, program] = args.as_slice() else {
        eprintln!("usage: harness <level> <program.so>");
        return ExitCode::from(2);
    };
    let Some(checks) = checks_for(level) else {
        eprintln!("unknown level: {level}");
        return ExitCode::from(2);
    };

    let results: Vec<CheckResult> = checks
        .iter()
        .map(|check| run_check(check, Path::new(program)))
        .collect();
    let report = Report {
        level: level.clone(),
        passed: results.iter().all(|r| r.passed),
        checks: results,
    };
    println!("{}", serde_json::to_string(&report).expect("report serializes"));
    ExitCode::SUCCESS
}
