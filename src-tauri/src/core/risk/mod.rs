pub fn score_package_risk(
    installed_size_bytes: u64,
    last_used_days_ago: Option<u32>,
    install_age_days: Option<u32>,
    protected: bool,
) -> f32 {
    if protected {
        return 0.98;
    }

    let size_factor = (installed_size_bytes as f32 / (1024.0 * 1024.0 * 1024.0)).min(1.0) * 0.20;
    let recency_factor = match last_used_days_ago {
        Some(days) if days > 180 => 0.10,
        Some(days) if days > 60 => 0.18,
        Some(days) if days > 14 => 0.28,
        Some(_) => 0.60,
        None => 0.45,
    };
    let age_factor = match install_age_days {
        Some(days) if days > 365 => 0.08,
        Some(days) if days > 90 => 0.12,
        Some(_) => 0.20,
        None => 0.15,
    };

    (0.15 + size_factor + recency_factor + age_factor).min(0.95)
}
