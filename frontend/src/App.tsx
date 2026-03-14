import { useEffect, useState } from "react";
import { Shell } from "./components/Shell";
import { Dashboard } from "./pages/Dashboard";
import { PackagesPage } from "./pages/Packages";
import Hardware from "./pages/Hardware";
import Optimizations from "./pages/Optimizations";
import { LogsPage } from "./pages/Logs";
import { SettingsPage } from "./pages/Settings";
import { OnboardingPage } from "./pages/Onboarding";
import HistoryPage from "./pages/History";
import ServicesPage from "./pages/Services";
import PluginsPage from "./pages/Plugins";
import { getDashboardSummary } from "./lib/api";

export type Page = "onboarding" | "dashboard" | "packages" | "hardware" | "optimizations" | "history" | "services" | "plugins" | "settings";

export default function App() {
  const [page, setPage] = useState<Page>("onboarding");
  const [dashboard, setDashboard] = useState<any>(null);

  const refreshData = async () => {
    try {
      const result = await getDashboardSummary();
      setDashboard(result);
    } catch {
      setDashboard({
        reclaimable_bytes: 0,
        safe_actions: 0,
        review_actions: 0,
        optimization_count: 0,
        package_counts: { native: 0, flatpak: 0, snap: 0, appimage: 0 }
      });
    }
  };

  useEffect(() => {
    refreshData();
  }, []);

  return (
    <Shell page={page} setPage={setPage}>
      {page === "onboarding" && <OnboardingPage onContinue={() => setPage("dashboard")} />}
      {page === "dashboard" && <Dashboard data={dashboard} onPackages={() => setPage("packages")} onRefresh={refreshData} />}
      {page === "packages" && <PackagesPage />}
      {page === "hardware" && <Hardware />}
      {page === "optimizations" && <Optimizations />}
      {page === "history" && <HistoryPage />}
      {page === "services" && <ServicesPage />}
      {page === "plugins" && <PluginsPage />}
      {page === "settings" && <SettingsPage />}
    </Shell>
  );
}
