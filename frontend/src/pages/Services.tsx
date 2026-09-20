import React, { useEffect, useState } from 'react';
import { listServices, previewToggleService, toggleService, ServiceRecord } from '../lib/api';

const Services: React.FC = () => {
  const [services, setServices] = useState<ServiceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'non-essential'>('non-essential');
  const [pending, setPending] = useState<{ name: string; enable: boolean; effect: string } | null>(null);

  const fetchServices = async () => {
    try {
      setLoading(true);
      const data = await listServices();
      setServices(data.sort((a, b) => a.name.localeCompare(b.name)));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchServices();
  }, []);

  // Step 1: ask what the change would do, without doing it. Disabling the
  // wrong unit is the failure mode this page exists to avoid.
  const handlePreview = async (name: string, enable: boolean) => {
    try {
      setToggling(name);
      setError(null);
      const effect = await previewToggleService(name, enable);
      setPending({ name, enable, effect });
    } catch (err) {
      setError(`Could not preview the change: ${err}`);
    } finally {
      setToggling(null);
    }
  };

  // Step 2: carry out the change the user just confirmed.
  const handleConfirm = async () => {
    if (!pending) return;
    try {
      setToggling(pending.name);
      setError(null);
      await toggleService(pending.name, pending.enable);
      setPending(null);
      await fetchServices();
    } catch (err) {
      setError(`Operation failed: ${err}`);
    } finally {
      setToggling(null);
    }
  };

  const filteredServices = filter === 'non-essential' 
    ? services.filter(s => s.non_essential)
    : services;

  if (loading) return <div className="page-container">Scanning services...</div>;

  return (
    <div className="page-container">
      <header className="page-header">
        <div>
          <h1 className="page-title">Service Auditor</h1>
          <p className="page-subtitle">Identify and disable non-essential background processes.</p>
        </div>
        <div className="filter-group">
          <button 
            className={`btn ${filter === 'non-essential' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => setFilter('non-essential')}
          >
            Non-Essential Only
          </button>
          <button 
            className={`btn ${filter === 'all' ? 'btn-primary' : 'btn-secondary'}`}
            onClick={() => setFilter('all')}
          >
            All Services
          </button>
        </div>
      </header>

      {error && <div className="card alert-error">{error}</div>}

      {pending && (
        <div className="card">
          <h2>Confirm service change</h2>
          <p>
            Luxor will <strong>{pending.effect}</strong>.
          </p>
          <div className="hint">
            Disabling a unit other services depend on can leave the system in a
            degraded state until it is re-enabled.
          </div>
          <div className="flex-gap inherit-font mt-12">
            <button
              className="btn btn-primary"
              disabled={toggling !== null}
              onClick={handleConfirm}
            >
              {toggling ? 'Applying...' : `Confirm ${pending.enable ? 'enable' : 'disable'}`}
            </button>
            <button
              className="btn btn-secondary"
              disabled={toggling !== null}
              onClick={() => setPending(null)}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="grid">
        {filteredServices.map((service) => (
          <div key={service.name} className={`card service-card ${service.non_essential ? 'highlight-warn' : ''}`}>
            <div className="flex-between">
              <div>
                <div className="flex-gap">
                  <span className="service-name">{service.name}</span>
                  <span className={`badge ${service.status === 'active' ? 'badge-success' : 'badge-neutral'}`}>
                    {service.status}
                  </span>
                  {service.non_essential && <span className="badge badge-warn">Non-Essential</span>}
                </div>
                <div className="service-desc">{service.description}</div>
                <div className="service-meta">Category: {service.category}</div>
              </div>
              <button
                className={`btn ${service.enabled ? 'btn-danger' : 'btn-success'}`}
                onClick={() => handlePreview(service.name, !service.enabled)}
                disabled={toggling === service.name}
              >
                {toggling === service.name ? '...' : (service.enabled ? 'Disable' : 'Enable')}
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Services;
