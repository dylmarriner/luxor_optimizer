import React, { useEffect, useState } from 'react';
import { listServices, toggleService, ServiceRecord } from '../lib/api';

const Services: React.FC = () => {
  const [services, setServices] = useState<ServiceRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'non-essential'>('non-essential');

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

  const handleToggle = async (name: string, enable: boolean) => {
    try {
      setToggling(name);
      await toggleService(name, enable);
      await fetchServices();
    } catch (err) {
      alert(`Operation failed: ${err}`);
    } finally {
      setToggling(null);
    }
  };

  const filteredServices = filter === 'non-essential' 
    ? services.filter(s => s.non_essential)
    : services;

  if (loading) return <div className="page-container">Scanning services...</div>;
  if (error) return <div className="page-container error-text">Error: {error}</div>;

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
                onClick={() => handleToggle(service.name, !service.enabled)}
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
