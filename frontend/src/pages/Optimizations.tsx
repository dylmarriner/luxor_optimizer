import { useEffect, useState } from "react";
import { runFullScan, applyOptimization, OptimizationRecommendation } from "../lib/api";
import { RiskBadge } from "../components/RiskBadge";

export default function Optimizations() {
  const [recommendations, setRecommendations] = useState<OptimizationRecommendation[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [applying, setApplying] = useState<string | null>(null);
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

  const handleApply = async (id: string) => {
    try {
      setApplying(id);
      setError(null);
      setSuccess(null);
      await applyOptimization(id);
      setSuccess(`Successfully applied optimization: ${id}`);
    } catch (err) {
      setError(`Failed to apply optimization: ${err}`);
    } finally {
      setApplying(null);
    }
  };

  const mapRisk = (score: number): "safe" | "review" | "expert" => {
    if (score >= 0.75) return "expert";
    if (score >= 0.35) return "review";
    return "safe";
  };

  if (loading) return <div className="page"><div className="card">Loading optimization recommendations...</div></div>;

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
        <div className="card"><p>No recommendations available at this time.</p></div>
      ) : (
        <div className="grid">
          {recommendations.map((rec) => (
            <div key={rec.id} className="card">
              <div className="flex-between">
                <h3>{rec.title}</h3>
                <button 
                  className="primary-btn" 
                  disabled={applying !== null}
                  onClick={() => handleApply(rec.id)}
                >
                  {applying === rec.id ? 'Applying...' : 'Apply'}
                </button>
              </div>
              <p>{rec.rationale}</p>
              <div className="flex-gap">
                <span><strong>Profile:</strong> {rec.profile}</span>
                <span>
                   <strong>Risk:</strong> {(rec.risk_score * 100).toFixed(0)}/100
                </span>
                <RiskBadge risk={mapRisk(rec.risk_score)} />
                <span>
                  <strong>Reversible:</strong> {rec.reversible ? '✓' : '✗'}
                </span>
                <span><strong>Root:</strong> {rec.requires_root ? 'Required' : 'Not required'}</span>
              </div>
              <div className="mt-12">
                <strong>Preview</strong>
                <ul className="compact">
                  {rec.command_preview.map((step) => (
                    <li key={step}>{step}</li>
                  ))}
                </ul>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
