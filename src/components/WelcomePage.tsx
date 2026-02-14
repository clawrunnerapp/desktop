interface WelcomePageProps {
  isConfigured: boolean;
  onStart: () => void;
}

export function WelcomePage({ isConfigured, onStart }: WelcomePageProps) {
  return (
    <div className="welcome-page">
      <div className="welcome-content">
        <div className="welcome-logo">&#129438;</div>
        <h1>OpenClaw Desktop</h1>
        <p className="welcome-version">v0.0.1</p>

        <p className="welcome-description">
          Your personal AI assistant that lives on your machine.
          <br />
          Talk to it through WhatsApp, Telegram, Slack, voice — or right here.
        </p>

        <button className="welcome-start-btn" onClick={onStart}>
          {isConfigured ? "Start gateway" : "Begin setup"}
        </button>
      </div>
    </div>
  );
}
