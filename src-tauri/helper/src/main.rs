use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{env, process::Command};

#[derive(Debug, Deserialize)]
struct HelperRequest {
    action: String,
    command: String,
    args: Vec<String>,
    dry_run: bool,
}

#[derive(Debug, Serialize)]
struct HelperResponse {
    status: String,
    stdout: String,
    stderr: String,
}

fn main() {
    match run() {
        Ok(response) => println!("{}", serde_json::to_string(&response).unwrap()),
        Err(err) => {
            let response = HelperResponse {
                status: "error".to_string(),
                stdout: String::new(),
                stderr: err.to_string(),
            };
            println!("{}", serde_json::to_string(&response).unwrap());
            std::process::exit(1);
        }
    }
}

fn run() -> Result<HelperResponse> {
    let payload = env::args().nth(1).ok_or_else(|| anyhow::anyhow!("missing helper payload"))?;
    let request: HelperRequest = serde_json::from_str(&payload)?;
    validate(&request)?;

    if request.dry_run {
        return Ok(HelperResponse {
            status: "dry-run".to_string(),
            stdout: format!("{} {:?}", request.command, request.args),
            stderr: String::new(),
        });
    }

    let output = Command::new(&request.command).args(&request.args).output()?;
    Ok(HelperResponse {
        status: if output.status.success() { "ok" } else { "failed" }.to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn validate(request: &HelperRequest) -> Result<()> {
    const ALLOWLIST: &[&str] = &["apt", "dnf", "pacman", "zypper", "flatpak", "snap", "systemctl", "rm", "cpupower", "sysctl"];

    if !ALLOWLIST.contains(&request.command.as_str()) {
        bail!("command not allowlisted: {}", request.command);
    }

    // Command-specific argument validation
    match request.command.as_str() {
        "rm" => {
            if request.args.iter().any(|arg| arg == "/" || arg.starts_with("/home/") || arg.contains("..")) {
                bail!("refusing dangerous or relative rm target");
            }
            if !request.args.iter().any(|arg| arg.starts_with("/var/cache/") || arg.starts_with("/tmp/") || arg.starts_with("/var/tmp/")) {
                if !request.args.iter().any(|arg| arg.contains(".cache/")) {
                     bail!("rm restricted to cache and temp directories");
                }
            }
        },
        "apt" | "dnf" | "pacman" | "zypper" => {
            let dangerous = ["update", "upgrade", "dist-upgrade", "full-upgrade"];
            if request.args.iter().any(|arg| dangerous.contains(&arg.as_str())) {
                bail!("bulk upgrade actions not allowed via helper");
            }
        },
        "systemctl" => {
            let allowed_sub = ["status", "is-active", "is-enabled", "enable", "disable", "start", "stop"];
            if !request.args.iter().any(|arg| allowed_sub.contains(&arg.as_str())) {
                bail!("unsupported systemctl subcommand");
            }
        },
        _ => {}
    }

    if request.action.trim().is_empty() {
        bail!("missing action name");
    }

    Ok(())
}
