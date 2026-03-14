use crate::models::ServiceRecord;
use anyhow::Result;
use std::process::Command;

pub struct ServiceAuditor;

impl ServiceAuditor {
    pub fn scan(&self) -> Result<Vec<ServiceRecord>> {
        let output = Command::new("systemctl")
            .args(["list-units", "--type=service", "--all", "--no-legend"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut services = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 { continue; }

            let name = parts[0].to_string();
            let status = parts[3].to_string(); // active/inactive
            let description = parts[4..].join(" ");

            let enabled = self.is_enabled(&name);
            let (non_essential, category) = self.classify(&name);

            services.push(ServiceRecord {
                name,
                description,
                status,
                enabled,
                non_essential,
                category,
            });
        }

        Ok(services)
    }

    fn is_enabled(&self, name: &str) -> bool {
        Command::new("systemctl")
            .args(["is-enabled", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn classify(&self, name: &str) -> (bool, String) {
        let blocklist = [
            ("bluetooth.service", "Communication"),
            ("cups.service", "Printing"),
            ("avahi-daemon.service", "Discovery"),
            ("geoclue.service", "Location"),
            ("unattended-upgrades.service", "Updates"),
            ("apport.service", "Telemetry"),
            ("whoopsie.service", "Telemetry"),
            ("tracker-miner-fs-3.service", "Indexing"),
        ];

        for (pattern, cat) in blocklist {
            if name.contains(pattern) {
                return (true, cat.to_string());
            }
        }

        (false, "System".to_string())
    }
}
