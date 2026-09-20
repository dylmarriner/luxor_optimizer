import { useEffect, useState } from "react";
import {
  applyOptimization,
  previewOptimization,
  runFullScan,
  type OptimizationRecommendation
} from "../lib/api";
import { RiskBadge } from "../components/RiskBadge";

/** Preview lines for one recommendation, kept per-id so several can be open. */
type PreviewState = Record<string, string[]>;

export default function Optimizations() {
  const [recommendations, setRecommendations] = useState<OptimizationRecommendation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [previews, setPreviews] = useState<PreviewState>({});
  const [success, setSuccess] = useState<string | null>(null);

  const fetchRecommendations = async () => {
    try {
      setLoading(true);
      const result = await runFullScan();
      setRecommendations(result.optimizations ?? []);
    } catch (err) {
      setError(`Failed to load recommendations: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchRecommendations();
  }, []);

  // Step 1: ask the backend what it would actually change, against this machine.
  const handlePreview = async (id: string) => {
    try {
      setBusy(id);
      setError(null);
      setSuccess(null);
      const steps = await previewOptimization(id);
      setPreviews((current) => ({ ...current, [id]: steps }));
    } catch (err) {
      setError(`Could not preview ${id}: ${err}`);
    } finally {
      setBusy(null);
    }
  };

  // Step 2: run it, having shown the user the validated steps above.
  const handleApply = async (id: string) => {
    try {
      setBusy(id);
      setError(null);
      await applyOptimization(id);
      setSuccess(`Applied ${id}. It is reversible from the History page.`);
      setPreviews((current) => {
        const next = { ...current };
        delete next[id];
        return next;
      });
    } catch (err) {
      setError(`Failed to apply ${id}: ${err}`);
    } finally {
      setBusy(null);
    }
  };

  const mapRisk = (score: number): "safe" | "review" | "expert" => {
    if (score >= 0.75) return "expert";
    if (score >= 0.35) return "review";
    return "safe";
  };

  if (loading) {
    return (
      <div className="page">
        <div className="card">Loading optimization recommendations...</div>
      </div>
    );
  }

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Optimization Recommendations</div>
          <h1>Evidence-based and reversible where possible</h1>
        </div>
      </header>

      {error && <div className="card alert-error">{error}</div>}
      {success && <div className="card alert-success">{success}</div>}

      {recommendations.length === 0 ? (
        <div className="card">
          <p>No recommendations available at this time.</p>
        </div>
      ) : (
        <div className="grid">
          {recommendations.map((rec) => {
            const preview = previews[rec.id];
            return (
              <div key={rec.id} className="card">
                <div className="flex-between">
                  <h3>{rec.title}</h3>
                  {rec.automatable ? (
                    <button
                      className="primary-btn"
                      disabled={busy !== null}
                      onClick={() => handlePreview(rec.id)}
                    >
                      {busy === rec.id && !preview ? "Checking..." : "Preview"}
                    </button>
                  ) : (
                    <span className="badge badge-neutral">Advisory</span>
                  )}
                </div>
                <p>{rec.rationale}</p>
                <div className="flex-gap">
                  <span>
                    <strong>Profile:</strong> {rec.profile}
                  </span>
                  <span>
                    <strong>Risk:</strong> {(rec.risk_score * 100).toFixed(0)}/100
                  </span>
                  <RiskBadge risk={mapRisk(rec.risk_score)} />
                  <span>
                    <strong>Reversible:</strong> {rec.reversible ? "✓" : "✗"}
                  </span>
                  <span>
                    <strong>Root:</strong> {rec.requires_root ? "Required" : "Not required"}
                  </span>
                </div>

                {!rec.automatable && (
                  <div className="mt-12">
                    <strong>Suggested steps</strong>
                    <ul className="compact">
                      {rec.command_preview.map((step) => (
                        <li key={step}>{step}</li>
                      ))}
                    </ul>
                    <div className="hint">
                      Luxor does not automate this one. Follow the steps yourself, or
                      check back once an apply path ships.
                    </div>
                  </div>
                )}

                {preview && (
                  <div className="mt-12">
                    <strong>This will change:</strong>
                    <ul className="compact">
                      {preview.map((step) => (
                        <li key={step}>{step}</li>
                      ))}
                    </ul>
                    <div className="flex-gap inherit-font mt-12">
                      <button
                        className="primary-btn"
                        disabled={busy !== null}
                        onClick={() => handleApply(rec.id)}
                      >
                        {busy === rec.id ? "Applying..." : "Apply these changes"}
                      </button>
                      <button
                        className="secondary-btn"
                        disabled={busy !== null}
                        onClick={() =>
                          setPreviews((current) => {
                            const next = { ...current };
                            delete next[rec.id];
                            return next;
                          })
                        }
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
