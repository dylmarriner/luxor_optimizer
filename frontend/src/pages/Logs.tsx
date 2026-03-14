const sample = `2026-03-11T14:12:00Z  scan.preview  session=af88  dry_run=true  subject=cache_scan  risk=0.12\n2026-03-11T14:12:04Z  cleanup.preview  session=af88  dry_run=true  subject=user_cache  reclaimable=321901223\n2026-03-11T14:12:09Z  recommendation  session=af88  source=flatpak  subject=unused_runtime  risk=0.21`;

export function LogsPage() {
  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Audit</div>
          <h1>Immutable-style action history</h1>
        </div>
      </header>
      <section className="card log-card">
        <pre>{sample}</pre>
      </section>
    </div>
  );
}
