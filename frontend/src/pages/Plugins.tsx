import React, { useEffect, useState } from 'react';
import { listPlugins, togglePlugin, PluginMetadata } from '../lib/api';

const Plugins: React.FC = () => {
  const [plugins, setPlugins] = useState<PluginMetadata[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [toggling, setToggling] = useState<string | null>(null);
  const [discoveryMode, setDiscoveryMode] = useState(false);

  const fetchPlugins = async () => {
    try {
      setLoading(true);
      const data = await listPlugins();
      setPlugins(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchPlugins();
  }, []);

  const handleToggle = async (id: string, enable: boolean) => {
    try {
      setToggling(id);
      await togglePlugin(id, enable);
      await fetchPlugins();
    } catch (err) {
      alert(`Operation failed: ${err}`);
    } finally {
      setToggling(null);
    }
  };

  const cloudPlugins = [
    { id: 'gaming-v1', name: 'Gaming Performance Pack', author: 'Luxor Community', version: '1.2.0', description: 'Optimizations for latency reduction and GPU scheduling in competitive games.', installed: false },
    { id: 'battery-v3', name: 'Ultra Battery Saver', author: 'PowerUser', version: '3.0.1', description: 'Aggressive power management for laptops on the go.', installed: false },
    { id: 'dev-tools', name: 'Developer Toolchain', author: 'CodeSmith', version: '0.9.5', description: 'Faster build times and Docker performance tweaks.', installed: false }
  ];

  if (loading) return <div className="page">Loading plugins...</div>;
  if (error) return <div className="page error-text">Error: {error}</div>;

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Extensions</div>
          <h1>{discoveryMode ? 'Cloud Discovery' : 'Plugin Architecture'}</h1>
        </div>
        <button className="primary-btn" onClick={() => setDiscoveryMode(!discoveryMode)}>
          {discoveryMode ? 'View Installed' : 'Discover Plugins'}
        </button>
      </header>

      {discoveryMode ? (
        <div className="grid grid-2">
          {cloudPlugins.map(plugin => (
            <div key={plugin.id} className="card plugin-card highlight-warn" style={{ borderLeftColor: '#ffd888' }}>
              <div className="flex-between">
                <div>
                  <div className="flex-gap">
                    <span className="plugin-name">{plugin.name}</span>
                    <span className="badge badge-warn">Cloud</span>
                  </div>
                  <div className="plugin-desc">{plugin.description}</div>
                  <div className="plugin-meta">Author: {plugin.author} • v{plugin.version}</div>
                </div>
                <button className="btn btn-success" onClick={() => alert('Cloud sync coming soon in v3!')}>
                  Install
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="history-list">
          {plugins.length === 0 ? (
            <div className="card empty-state">
              <p>No optimization plugins installed.</p>
              <p className="compact">Install plugins to <code>~/.config/luxor/plugins/</code> to extend capabilities.</p>
            </div>
          ) : (
            plugins.map((plugin) => (
              <div key={plugin.id} className="card plugin-card">
                <div className="flex-between">
                  <div>
                    <div className="flex-gap">
                      <span className="plugin-name">{plugin.name}</span>
                      <span className="badge badge-neutral">v{plugin.version}</span>
                    </div>
                    <div className="plugin-desc">{plugin.description}</div>
                    <div className="plugin-meta">
                      Author: {plugin.author} • {plugin.optimization_count} Optimizations
                    </div>
                  </div>
                  <button
                    className={`btn ${plugin.enabled ? 'btn-danger' : 'btn-success'}`}
                    onClick={() => handleToggle(plugin.id, !plugin.enabled)}
                    disabled={toggling === plugin.id}
                  >
                    {toggling === plugin.id ? '...' : (plugin.enabled ? 'Disable' : 'Enable')}
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};

export default Plugins;
