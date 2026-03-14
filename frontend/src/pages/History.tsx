import React, { useEffect, useState } from 'react';
import { getAuditEvents, rollbackOptimization, analyzeAuditEvent, AuditEvent } from '../lib/api';

const History: React.FC = () => {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [undoing, setUndoing] = useState<string | null>(null);
  const [analyzing, setAnalyzing] = useState<string | null>(null);
  const [analysisResult, setAnalysisResult] = useState<{ [id: string]: string }>({});

  const fetchEvents = async () => {
    try {
      setLoading(true);
      const data = await getAuditEvents();
      setEvents(data);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEvents();
  }, []);

  const handleRollback = async (id: string) => {
    if (!confirm('Are you sure you want to undo this optimization?')) return;
    try {
      setUndoing(id);
      await rollbackOptimization(id);
      await fetchEvents();
    } catch (err) {
      alert(`Rollback failed: ${err}`);
    } finally {
      setUndoing(null);
    }
  };

  const handleAnalyze = async (id: string) => {
    try {
      setAnalyzing(id);
      const result = await analyzeAuditEvent(id);
      setAnalysisResult(prev => ({ ...prev, [id]: result }));
    } catch (err) {
      alert(`Analysis failed: ${err}`);
    } finally {
      setAnalyzing(null);
    }
  };

  if (loading) return <div className="page">Loading history...</div>;
  if (error) return <div className="page">Error: {error}</div>;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Audit Logs</div>
          <h1>Optimization History</h1>
        </div>
      </header>

      <div className="history-list">
        {events.length === 0 ? (
          <div className="card">No optimization history found.</div>
        ) : (
          events.map((event) => (
            <div key={event.event_id} className="card history-card">
              <div className="history-header">
                <div>
                  <span className="badge badge-success">{event.action_type}</span>
                  <span className="ml-12" style={{ color: '#7f93b8' }}>{new Date(event.ts_utc).toLocaleString()}</span>
                </div>
                <div className="flex-gap">
                   <button 
                    className="btn btn-primary"
                    onClick={() => handleAnalyze(event.event_id)}
                    disabled={analyzing === event.event_id}
                  >
                    {analyzing === event.event_id ? 'Analyzing...' : 'Analyze with AI'}
                  </button>
                  <button 
                    className="btn btn-danger" 
                    onClick={() => handleRollback(event.event_id)}
                    disabled={undoing === event.event_id}
                  >
                    {undoing === event.event_id ? 'Undoing...' : 'Undo Change'}
                  </button>
                </div>
              </div>
              
              <div className="grid grid-2 mt-12">
                <div>
                  <div className="label">Target</div>
                  <div className="value">{event.target}</div>
                </div>
                <div>
                  <div className="label">Impact</div>
                  <div className="value-content">Score: {event.impact_score.toFixed(1)}</div>
                </div>
              </div>

              {analysisResult[event.event_id] && (
                <div className="mt-12 p-12" style={{ background: 'rgba(124, 156, 255, 0.1)', borderRadius: '8px', border: '1px solid rgba(124, 156, 255, 0.3)' }}>
                  <div className="label" style={{ color: '#7c9cff', marginBottom: '8px' }}>AI Insights</div>
                  <div style={{ whiteSpace: 'pre-wrap', fontSize: '14px', color: '#d4def2' }}>
                    {analysisResult[event.event_id]}
                  </div>
                </div>
              )}

              <div className="history-footer">
                <details>
                  <summary style={{ color: '#7f93b8', cursor: 'pointer', fontSize: '13px' }}>View Technical Details</summary>
                  <div className="grid grid-2 mt-12">
                    <div>
                      <div className="label">Event ID</div>
                      <div className="value" style={{ fontSize: '11px', fontFamily: 'monospace' }}>{event.event_id}</div>
                    </div>
                    <div>
                      <div className="label">Checksum</div>
                      <div className="value" style={{ fontSize: '11px', fontFamily: 'monospace' }}>{event.event_hash.substring(0, 16)}...</div>
                    </div>
                  </div>
                </details>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default History;
