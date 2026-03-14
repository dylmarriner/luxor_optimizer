import { useEffect, useState } from "react";
import { RiskBadge } from "../components/RiskBadge";
import { runFullScan } from "../lib/api";

interface PackageRecord {
  name: string;
  source: "Native" | "Flatpak" | "Snap" | "AppImage";
  installed_size_bytes: number;
  criticality: string;
  last_used_days_ago?: number;
  rationale: string[];
  risk_score: number;
}

function formatGiB(bytes: number) {
  return `${(bytes / (1024 ** 3)).toFixed(2)} GiB`;
}

function mapRisk(score: number): "safe" | "review" | "expert" {
  if (score >= 0.75) return "expert";
  if (score >= 0.35) return "review";
  return "safe";
}

export function PackagesPage() {
  const [rows, setRows] = useState<PackageRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    runFullScan()
      .then((result: any) => setRows(result.packages ?? []))
      .catch((err) => setError(`Failed to load package inventory: ${err}`))
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Package Management Center</div>
          <h1>Native, Flatpak, Snap, AppImage</h1>
        </div>
      </header>
      {loading && <section className="card">Loading package inventory...</section>}
      {error && <section className="card" style={{ color: "#ffabab" }}>{error}</section>}
      {!loading && !error && (
      <section className="card">
        <table className="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Source</th>
              <th>Installed Size</th>
              <th>Criticality</th>
              <th>Why surfaced</th>
              <th>Risk</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.name}>
                <td>{row.name}</td>
                <td>{row.source.toLowerCase()}</td>
                <td>{formatGiB(row.installed_size_bytes)}</td>
                <td>{row.criticality}</td>
                <td>{row.rationale.join(" • ")}</td>
                <td>
                  <RiskBadge risk={mapRisk(row.risk_score)} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
      )}
    </div>
  );
}
