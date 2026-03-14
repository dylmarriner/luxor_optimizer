export function SettingsPage() {
  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Policy & Privacy</div>
          <h1>Exclusions, scheduling, retention</h1>
        </div>
      </header>
      <section className="card settings-list">
        <label><input type="checkbox" defaultChecked /> Safe Mode default</label>
        <label><input type="checkbox" defaultChecked /> Redact usernames in exported audits</label>
        <label><input type="checkbox" defaultChecked /> Keep dry-run previews for 30 days</label>
        <label><input type="checkbox" /> Enable journald mirroring</label>
        <label><input type="checkbox" /> Enable OpenTelemetry export adapter</label>
      </section>
    </div>
  );
}
