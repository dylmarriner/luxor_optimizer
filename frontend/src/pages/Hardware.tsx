import { useEffect, useState } from "react";
import { detectSystemProfile, SystemProfile } from "../lib/api";

export default function Hardware() {
  const [profile, setProfile] = useState<SystemProfile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchProfile = async () => {
      try {
        setLoading(true);
        const result = await detectSystemProfile();
        setProfile(result);
      } catch (err) {
        setError(`Failed to detect system profile: ${err}`);
      } finally {
        setLoading(false);
      }
    };

    fetchProfile();
  }, []);

  if (loading) return <div className="page"><div className="card">Loading system profile...</div></div>;
  if (error) return <div className="page"><div className="card alert-error">{error}</div></div>;
  if (!profile) return <div className="page"><div className="card">No profile data</div></div>;

  const ram_gb = (profile.ram_bytes / (1024 * 1024 * 1024)).toFixed(1);
  const uptime_days = (profile.uptime_seconds / 86400).toFixed(1);
  const total_disk_bytes = profile.disks.reduce((acc, d) => acc + d.total_bytes, 0);
  const disk_gb = (total_disk_bytes / (1024 * 1024 * 1024)).toFixed(0);

  return (
    <div className="page">
      <header className="page-header">
        <div>
          <div className="eyebrow">Hardware & OS Profile</div>
          <h1>Platform detected state</h1>
        </div>
      </header>

      <div className="grid grid-2">
        <section className="card">
          <h2>Operating System</h2>
          <div className="settings-list">
            <div className="flex-between"><span>Distro</span> <span>{profile.distro} {profile.version}</span></div>
            <div className="flex-between"><span>Kernel</span> <span>{profile.kernel}</span></div>
            <div className="flex-between"><span>Desktop</span> <span>{profile.desktop_environment ?? "Unknown"}</span></div>
            <div className="flex-between"><span>Display</span> <span>{profile.display_server ?? "Unknown"}</span></div>
            <div className="flex-between"><span>Init</span> <span>{profile.init_system}</span></div>
            <div className="flex-between"><span>Uptime</span> <span>{uptime_days} days</span></div>
          </div>
        </section>

        <section className="card">
          <h2>Hardware</h2>
          <div className="settings-list">
            <div className="flex-between"><span>CPU</span> <span>{profile.cpu_model}</span></div>
            <div className="flex-between"><span>GPU</span> <span>{profile.gpu_model}</span></div>
            <div className="flex-between"><span>Threads</span> <span>{profile.cores} / {profile.threads}</span></div>
            <div className="flex-between"><span>RAM</span> <span>{ram_gb} GB</span></div>
            <div className="flex-between"><span>Disk</span> <span>{disk_gb} GB</span></div>
            {profile.power_profile && <div className="flex-between"><span>Power</span> <span>{profile.power_profile}</span></div>}
          </div>
        </section>
      </div>

      <section className="card">
        <h2>Disk Inventory</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Mount</th>
              <th>Type</th>
              <th>Total</th>
              <th>Free</th>
              <th>Kind</th>
            </tr>
          </thead>
          <tbody>
            {profile.disks.map(disk => (
              <tr key={disk.mount_point}>
                <td>{disk.mount_point}</td>
                <td>{disk.fs_type}</td>
                <td>{(disk.total_bytes / (1024 ** 3)).toFixed(1)} GB</td>
                <td>{(disk.available_bytes / (1024 ** 3)).toFixed(1)} GB</td>
                <td>{disk.kind} {disk.is_ssd ? '(SSD)' : ''}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
