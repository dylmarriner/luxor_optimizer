import { useState } from "react";
import { applySafeCleanup } from "../lib/api";
import { StatCard } from "../components/StatCard";

function formatGiB(bytes: number | undefined) {
  if (!bytes) return "0 GiB";
  return `${(bytes / (1024 ** 3)).toFixed(2)} GiB`;
}

export function Dashboard({ data, onPackages, onRefresh }: { data: any; onPackages: () => void; onRefresh: () => void }) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const handleQuickApply = async () => {
    try {
      setBusy(true);
      setMessage(null);
      const reclaimed = await applySafeCleanup() as number;
      setMessage(`Successfully reclaimed ${formatGiB(reclaimed)}`);
      onRefresh();
    } catch (err) {
      setMessage(`Cleanup failed: ${err}`);
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
            onClick={handleQuickApply}
          >
            {busy ? 'Cleaning...' : 'Apply All Safe Actions'}
          </button>
          <button className="secondary-btn" onClick={onPackages}>
            Open Package Center
          </button>
        </div>
      </header>
      
      {message && (
        <div className={`card ${message.includes('failed') ? 'alert-error' : 'alert-success'}`}>
          {message}
        </div>
      )}

      <div className="grid grid-4">
        <StatCard
          label="Reclaimable"
          value={formatGiB(data?.reclaimable_bytes)}
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
             <div className="label">Native (APT)</div>
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
