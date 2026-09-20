import { useEffect, useState } from "react";
import { getPolicy, type PolicyConfig } from "../lib/api";

/**
 * Policy view.
 *
 * These were previously unbound checkboxes with `defaultChecked` hardcoded, so
 * the page showed the same state regardless of the real configuration and
 * clicking them changed nothing. Until policy editing is implemented, this
 * reports what is actually in force rather than implying control that does not
 * exist.
 */
export function SettingsPage() {
  const [policy, setPolicy] = useState<PolicyConfig | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getPolicy()
      .then(setPolicy)
      .catch((err) => setError(`Could not read policy: ${err}`));
  }, []);

  const Row = ({ label, value, note }: { label: string; value: boolean; note: string }) => (
    <div className="stat-item">
      <div className="flex-between">
        <span>{label}</span>
        <span className={`badge ${value ? "badge-success" : "badge-neutral"}`}>
          {value ? "on" : "off"}
        </span>
      </div>
      <div className="hint">{note}</div>
    </div>
  );

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Policy &amp; Privacy</div>
          <h1>Current configuration</h1>
        </div>
      </header>

      {error && <div className="card alert-error">{error}</div>}

      {policy && (
        <>
          <section className="card settings-list">
            <Row
              label="Safe mode default"
              value={policy.safe_mode_default}
              note="Destructive actions require an explicit preview and approval."
            />
            <Row
              label="Redact usernames in exported audits"
              value={policy.redact_usernames}
              note="Home directory paths are rewritten before an audit bundle leaves the machine."
            />
            <Row
              label="journald mirroring"
              value={policy.journald_mirror}
              note="Not yet implemented."
            />
            <Row
              label="OpenTelemetry export"
              value={policy.otel_export}
              note="Not yet implemented."
            />
            <div className="stat-item">
              <div className="flex-between">
                <span>Audit retention</span>
                <span className="badge badge-neutral">{policy.retention_days} days</span>
              </div>
              <div className="hint">Retention is recorded but not yet enforced by a pruning job.</div>
            </div>
          </section>

          <section className="card">
            <h2>Protected paths</h2>
            <div className="hint">
              Never deleted. A cache directory nested inside one of these stays
              eligible, resolved by the more specific rule.
            </div>
            <ul className="compact">
              {policy.protected_paths.map((path) => (
                <li key={path}>
                  <code>{path}</code>
                </li>
              ))}
            </ul>
          </section>

          <section className="card">
            <h2>Protected applications</h2>
            <div className="hint">Never recommended for removal.</div>
            <ul className="compact">
              {policy.protected_apps.map((app) => (
                <li key={app}>
                  <code>{app}</code>
                </li>
              ))}
            </ul>
          </section>

          <div className="card">
            <div className="hint">
              Editing policy from the UI is not implemented yet. These values come
              from the built-in default policy.
            </div>
          </div>
        </>
      )}
    </div>
  );
}
