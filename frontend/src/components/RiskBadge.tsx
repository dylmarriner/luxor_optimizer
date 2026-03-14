export function RiskBadge({ risk }: { risk: "safe" | "review" | "expert" }) {
  return <span className={`risk risk-${risk}`}>{risk.toUpperCase()}</span>;
}
