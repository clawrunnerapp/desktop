export function WebUIView() {
  return (
    <iframe
      className="webui-iframe"
      src="http://localhost:18789/a2ui/"
      sandbox="allow-scripts allow-same-origin allow-forms"
      title="OpenClaw Web UI"
    />
  );
}
