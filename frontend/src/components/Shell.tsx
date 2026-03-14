import type { ReactNode } from "react";
import type { Page } from "../App";

export function Shell({
  children,
  page,
  setPage
}: {
  children: ReactNode;
  page: Page;
  setPage: (page: Page) => void;
}) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div>
          <div className="brand">Luxor</div>
          <div className="subbrand">Linux Optimizer</div>
        </div>
        <nav className="nav">
          {[
            ["onboarding", "Onboarding"],
            ["dashboard", "Dashboard"],
            ["packages", "Packages"],
            ["hardware", "Hardware"],
            ["optimizations", "Optimizations"],
            ["history", "History"],
            ["services", "Services"],
            ["plugins", "Plugins"],
            ["settings", "Settings"]
          ].map(([id, label]) => (
            <button
              key={id}
              className={page === id ? "nav-btn active" : "nav-btn"}
              onClick={() => setPage(id as Page)}
            >
              {label}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">{children}</main>
    </div>
  );
}
