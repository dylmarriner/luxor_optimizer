//! Entry point for the root helper.
//!
//! Reads one [`HelperRequest`] from stdin, executes it, writes one
//! [`HelperResponse`] to stdout. The request arrives on stdin rather than argv
//! so that action parameters are not exposed in `/proc/<pid>/cmdline` to other
//! users on the machine.

use luxor_ipc::{HelperRequest, HelperResponse, Status};
use luxor_helper::execute::execute;
use std::io::{self, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let response = match run() {
        Ok(response) => response,
        Err(err) => HelperResponse {
            status: Status::Error,
            effect: String::new(),
            before: None,
            after: None,
            // `{:#}` renders the anyhow context chain on one line.
            message: Some(format!("{err:#}")),
        },
    };

    let failed = response.status == Status::Error;
    let encoded = serde_json::to_string(&response)
        .unwrap_or_else(|_| r#"{"status":"error","effect":"","message":"response encoding failed"}"#.to_string());

    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{encoded}");
    let _ = stdout.flush();

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run() -> anyhow::Result<HelperResponse> {
    let mut payload = String::new();
    io::stdin().read_to_string(&mut payload)?;

    // Deserialization *is* the validation: every parameter type in the action
    // contract rejects unsafe values during parsing, so a request that parses
    // is a request that is safe to run.
    let request: HelperRequest = serde_json::from_str(payload.trim())?;
    execute(&request.action, request.dry_run)
}
