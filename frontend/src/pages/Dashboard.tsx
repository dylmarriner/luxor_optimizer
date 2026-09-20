import { useState } from "react";
import {
  applySafeCleanup,
  previewSafeCleanup,
  type CleanupOutcome,
  type CleanupPlan
} from "../lib/api";
import { StatCard } from "../components/StatCard";

function formatBytes(bytes: number | undefined) {
  if (!bytes) return "0 B";
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value < 10 && unit > 0 ? 2 : 0)} ${units[unit]}`;
}

export function Dashboard({
  data,
  onPackages,
  onRefresh
}: {
  data: any;
  onPackages: () => void;
  onRefresh: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [plan, setPlan] = useState<CleanupPlan | null>(null);
  const [outcome, setOutcome] = useState<CleanupOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Step 1: ask the backend exactly what it would delete. Nothing is touched.
  const handlePreview = async () => {
    try {
      setBusy(true);
      setError(null);
      setOutcome(null);
      setPlan(await previewSafeCleanup());
    } catch (err) {
      setError(`Could not build a cleanup plan: ${err}`);
    } finally {
      setBusy(false);
    }
  };

  // Step 2: run only the plan the user just read, identified by its token.
  const handleApprove = async () => {
    if (!plan) return;
    try {
      setBusy(true);
      setError(null);
      const result = await applySafeCleanup(plan.token);
      setOutcome(result);
      setPlan(null);
      onRefresh();
    } catch (err) {
      setError(`Cleanup failed: ${err}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">System Summary</div>
          <h1>System Health Dashboard</h1>
        </div>
        <div className="flex-gap inherit-font">
          <button
            className="primary-btn"
            disabled={busy || data?.safe_actions === 0}
            onClick={handlePreview}
          >
            {busy && !plan ? "Scanning..." : "Preview Safe Cleanup"}
          </button>
          <button className="secondary-btn" onClick={onPackages}>
            Open Package Center
          </button>
        </div>
      </header>

      {error && <div className="card alert-error">{error}</div>}

      {plan && (
        <section className="card">
          <h2>Review before deleting</h2>
          {plan.targets.length === 0 ? (
            <p>Nothing is eligible for unattended cleanup right now.</p>
          ) : (
            <>
              <p>
                This will permanently delete the <strong>contents</strong> of the
                directories below — {formatBytes(plan.total_bytes)} across{" "}
                {plan.targets.length} location{plan.targets.length === 1 ? "" : "s"}.
                Deleted cache files cannot be recovered.
              </p>
              <ul className="compact">
                {plan.targets.map((target) => (
                  <li key={target.id}>
                    <code>{target.path}</code> — {formatBytes(target.bytes)}
                    <div className="hint">{target.rationale}</div>
                  </li>
                ))}
              </ul>
              <div className="flex-gap inherit-font mt-12">
                <button className="primary-btn" disabled={busy} onClick={handleApprove}>
                  {busy ? "Deleting..." : `Delete ${formatBytes(plan.total_bytes)}`}
                </button>
                <button className="secondary-btn" disabled={busy} onClick={() => setPlan(null)}>
                  Cancel
                </button>
              </div>
            </>
          )}
        </section>
      )}

      {outcome && (
        <section className="card alert-success">
          <div>Reclaimed {formatBytes(outcome.reclaimed_bytes)}.</div>
          {outcome.skipped.length > 0 && (
            <>
              <div className="mt-12">
                <strong>Skipped {outcome.skipped.length}:</strong>
              </div>
              <ul className="compact">
                {outcome.skipped.map((skip) => (
                  <li key={skip.path}>
                    <code>{skip.path}</code> — {skip.reason}
                  </li>
                ))}
              </ul>
            </>
          )}
        </section>
      )}

      <div className="grid grid-4">
        <StatCard
          label="Reclaimable"
          value={formatBytes(data?.reclaimable_bytes)}
          hint="Total space that can be safely freed"
        />
        <StatCard label="Quick Actions" value={String(data?.safe_actions ?? 0)} hint="Safe automated cleanups" />
        <StatCard label="Review Required" value={String(data?.review_actions ?? 0)} hint="Actions needing manual check" />
        <StatCard
          label="Optimizations"
          value={String(data?.optimization_count ?? 0)}
          hint="Available system-level improvements"
        />
      </div>

      <section className="card">
        <h2>Package Ecosystem</h2>
        <div className="grid grid-4 compact">
          <div className="stat-item">
            <div className="label">Native</div>
            <div className="stat-value">{data?.package_counts?.native ?? 0}</div>
          </div>
          <div className="stat-item">
            <div className="label">Flatpak</div>
            <div className="stat-value">{data?.package_counts?.flatpak ?? 0}</div>
          </div>
          <div className="stat-item">
            <div className="label">Snap</div>
            <div className="stat-value">{data?.package_counts?.snap ?? 0}</div>
          </div>
          <div className="stat-item">
            <div className="label">AppImage</div>
            <div className="stat-value">{data?.package_counts?.appimage ?? 0}</div>
          </div>
        </div>
      </section>
    </div>
  );
}
