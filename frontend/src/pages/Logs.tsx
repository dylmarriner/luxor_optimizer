import { useEffect, useState } from "react";
import { verifyAuditChain, type ChainVerification } from "../lib/api";

/**
 * Audit integrity view.
 *
 * This page previously rendered a hardcoded three-line sample that looked like
 * real log output. History already lists events, so rather than duplicating it,
 * this shows the thing nothing else did: whether the hash chain actually holds.
 */
export function LogsPage() {
  const [result, setResult] = useState<ChainVerification | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const verify = async () => {
    try {
      setBusy(true);
      setError(null);
      setResult(await verifyAuditChain());
    } catch (err) {
      setError(`Could not verify the audit chain: ${err}`);
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    verify();
  }, []);

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Audit</div>
          <h1>Chain integrity</h1>
        </div>
        <button className="secondary-btn" disabled={busy} onClick={verify}>
          {busy ? "Verifying..." : "Re-verify"}
        </button>
      </header>

      {error && <div className="card alert-error">{error}</div>}

      {result && (
        <section className={`card ${result.intact ? "alert-success" : "alert-error"}`}>
          <h2>
            {result.intact
              ? "Chain verified"
              : `Chain verification failed (${result.broken_at.length} problem${
                  result.broken_at.length === 1 ? "" : "s"
                })`}
          </h2>
          <p>
            Recomputed {result.events_checked} event
            {result.events_checked === 1 ? "" : "s"} from genesis, checking each
            record's hash and its link to the one before it.
          </p>

          {result.legacy_unverifiable > 0 && (
            <div className="hint">
              {result.legacy_unverifiable} record
              {result.legacy_unverifiable === 1 ? " was" : "s were"} written under an
              earlier field naming and cannot be re-hashed under the current schema.
              Their links are still checked; they are not evidence of tampering.
            </div>
          )}

          {result.broken_at.length > 0 && (
            <ul className="compact mt-12">
              {result.broken_at.map((issue) => (
                <li key={`${issue.position}-${issue.event_id}`}>
                  <code>{issue.event_id}</code> at position {issue.position}
                  <div className="hint">{issue.reason}</div>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {result?.events_checked === 0 && (
        <div className="card">
          <p>No audit events recorded yet. Run a scan or apply a change.</p>
        </div>
      )}
    </div>
  );
}
