//! Command-line companion.
//!
//! Every subcommand is a thin call into `luxor-core`, so the CLI and the
//! desktop app cannot disagree about what an action does. The same rule as the
//! GUI applies here: anything privileged or destructive previews by default and
//! needs `--apply` to act.

use anyhow::{bail, Context, Result};
use luxor_core::core::{
    audit::AuditLogger,
    cleanup::engine::CleanupEngine,
    detect::system::SystemDetector,
    optimizations::advisor::OptimizationAdvisor,
    packages::manager::PackageManagerInventory,
    policy::PolicyEngine,
};
use luxor_core::models::CleanupDisposition;
use std::process::ExitCode;

const USAGE: &str = "\
luxor — Linux desktop optimizer

USAGE:
    luxor <COMMAND> [OPTIONS]

COMMANDS:
    profile              Show detected hardware and system profile
    scan                 Scan for reclaimable space and recommendations
    clean [--apply]      Preview cleanup; --apply deletes after showing the plan
    optimize <ID> [--apply]
                         Preview an optimization; --apply runs it
    packages             List installed packages across all ecosystems
    capabilities         Show what this machine actually supports
    audit [--verify]     Show recent audit events; --verify checks the chain
    help                 Show this message

Previewing is the default everywhere. Nothing is changed without --apply.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<()> {
    let Some(command) = args.first().map(String::as_str) else {
        print!("{USAGE}");
        return Ok(());
    };
    let apply = args.iter().any(|a| a == "--apply");

    match command {
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        "profile" => profile(),
        "scan" => scan(),
        "clean" => clean(apply),
        "optimize" => {
            let id = args
                .get(1)
                .filter(|a| !a.starts_with("--"))
                .context("optimize needs an optimization id; run `luxor scan` to list them")?;
            optimize(id, apply)
        }
        "packages" => packages(),
        "capabilities" => capabilities(),
        "audit" => audit(args.iter().any(|a| a == "--verify")),
        other => bail!("unknown command {other:?}; run `luxor help`"),
    }
}

fn profile() -> Result<()> {
    let profile = SystemDetector::default().detect()?;
    println!("{} {} ({})", profile.distro, profile.version, profile.kernel);
    println!("CPU     {} ({}c/{}t)", profile.cpu_model, profile.cores, profile.threads);
    println!("GPU     {}", profile.gpu_model);
    println!("Memory  {} RAM, {} swap", human(profile.ram_bytes), human(profile.swap_bytes));
    println!("Init    {}", profile.init_system);
    if let Some(de) = &profile.desktop_environment {
        println!("Desktop {de} on {}", profile.display_server.as_deref().unwrap_or("unknown"));
    }
    println!("Package managers: {}", profile.package_managers.join(", "));
    println!();
    for disk in &profile.disks {
        println!(
            "  {:<24} {:<8} {:>10} free of {:>10}  {}",
            disk.mount_point,
            disk.fs_type,
            human(disk.available_bytes),
            human(disk.total_bytes),
            if disk.is_ssd { "SSD" } else { "rotational/unknown" }
        );
    }
    Ok(())
}

fn scan() -> Result<()> {
    let policy = PolicyEngine::default_policy();
    let profile = SystemDetector::default().detect()?;
    let findings = CleanupEngine::new(policy.clone()).scan(&profile)?;
    let recommendations = OptimizationAdvisor::new(policy).recommend(&profile)?;

    let total: u64 = findings.iter().map(|f| f.bytes).sum();
    println!("Reclaimable: {} across {} findings\n", human(total), findings.len());
    for f in &findings {
        println!(
            "  [{:?}] {:<28} {:>10}  {}",
            f.disposition,
            f.label,
            human(f.bytes),
            f.path
        );
    }

    println!("\nRecommendations:");
    for rec in &recommendations {
        println!(
            "  {:<24} risk {:.2}  {}{}",
            rec.id,
            rec.risk_score,
            rec.title,
            if rec.automatable { "" } else { "  (advisory only)" }
        );
    }
    Ok(())
}

fn clean(apply: bool) -> Result<()> {
    let policy = PolicyEngine::default_policy();
    let profile = SystemDetector::default().detect()?;
    let engine = CleanupEngine::new(policy);

    let targets: Vec<_> = engine
        .scan(&profile)?
        .into_iter()
        .filter(|f| matches!(f.disposition, CleanupDisposition::SafeAuto))
        .collect();

    if targets.is_empty() {
        println!("Nothing is eligible for unattended cleanup.");
        return Ok(());
    }

    let total: u64 = targets.iter().map(|f| f.bytes).sum();
    println!("This would permanently delete the contents of:\n");
    for f in &targets {
        println!("  {:<48} {:>10}", f.path, human(f.bytes));
    }
    println!("\nTotal: {}", human(total));

    if !apply {
        println!("\nPreview only. Re-run with --apply to delete.");
        return Ok(());
    }

    let audit = AuditLogger::new()?;
    let mut reclaimed = 0u64;
    for f in &targets {
        match engine.apply(f) {
            Ok(()) => {
                reclaimed += f.bytes;
                audit.record_event(
                    "apply-cleanup",
                    &f.path,
                    serde_json::json!({ "bytes": f.bytes }),
                    serde_json::json!({ "bytes": 0 }),
                    serde_json::json!({ "finding": f.id, "via": "cli" }),
                    0.10,
                    0.4,
                )?;
            }
            Err(err) => eprintln!("  skipped {}: {err}", f.path),
        }
    }
    println!("\nReclaimed {}.", human(reclaimed));
    Ok(())
}

fn optimize(id: &str, apply: bool) -> Result<()> {
    let advisor = OptimizationAdvisor::new(PolicyEngine::default_policy());

    let steps = advisor.preview(id)?;
    println!("{id} would:");
    for step in &steps {
        println!("  - {step}");
    }

    if !apply {
        println!("\nPreview only. Re-run with --apply to make these changes.");
        return Ok(());
    }

    advisor.apply(id, &AuditLogger::new()?)?;
    println!("\nApplied. Reversible via the recorded audit event.");
    Ok(())
}

fn capabilities() -> Result<()> {
    let platform = luxor_core::core::platform::current();
    println!("Platform: {}\n", platform.id());
    for (capability, support) in platform.probe() {
        let mark = if support.is_available() { "yes" } else { " no" };
        println!("  [{mark}] {:<28} {}", capability.label(), support.explain());
    }
    Ok(())
}

fn packages() -> Result<()> {
    let policy = PolicyEngine::default_policy();
    let profile = SystemDetector::default().detect()?;
    let packages = PackageManagerInventory::new(policy).discover(&profile)?;

    println!("{} packages\n", packages.len());
    for pkg in &packages {
        println!(
            "  {:<10?} {:<40} {:>10}  risk {:.2}",
            pkg.source,
            pkg.name,
            pkg.installed_size_bytes.map(human).unwrap_or_else(|| "unknown".to_string()),
            pkg.risk_score
        );
    }
    Ok(())
}

fn audit(verify: bool) -> Result<()> {
    let logger = AuditLogger::new()?;

    if verify {
        let result = logger.verify_chain()?;
        println!(
            "Checked {} events: {}",
            result.events_checked,
            if result.intact { "chain intact" } else { "CHAIN BROKEN" }
        );
        if result.legacy_unverifiable > 0 {
            println!(
                "  {} record(s) predate the current schema and cannot be re-hashed.",
                result.legacy_unverifiable
            );
        }
        for issue in &result.broken_at {
            println!("  {} at {}: {}", issue.event_id, issue.position, issue.reason);
        }
        if !result.intact {
            bail!("audit chain verification failed");
        }
        return Ok(());
    }

    for event in logger.get_events()?.iter().take(25) {
        println!(
            "{}  {:<24} {:<32} risk {:.2}{}",
            event.ts_utc,
            event.action_type,
            event.target,
            event.risk_score,
            if event.dry_run { "  (dry run)" } else { "" }
        );
    }
    Ok(())
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.2} {}", UNITS[unit])
    }
}
