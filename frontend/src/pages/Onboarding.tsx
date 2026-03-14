export function OnboardingPage({ onContinue }: { onContinue: () => void }) {
  return (
    <div className="page">
      <div className="hero">
        <div className="eyebrow">Guided Safe Mode</div>
        <h1>Clean up Linux without playing roulette with your system.</h1>
        <p>
          Luxor starts in dry-run mode, explains permissions, previews every destructive action,
          and separates safe cleanup from review-only suggestions.
        </p>
        <ol className="steps">
          <li>Compatibility check</li>
          <li>Permission explanation</li>
          <li>Safe Mode defaults</li>
          <li>First scan consent</li>
          <li>Post-scan recommendations</li>
        </ol>
        <button className="primary-btn" onClick={onContinue}>
          Start first scan
        </button>
      </div>
    </div>
  );
}
