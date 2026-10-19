import { useCallback, useEffect, useState, type ReactNode } from "react";
import type { ViteServerInfo } from "@shared/types";

const REFRESH_MS = 2000;

function serverLabel(server: ViteServerInfo): string {
  const parts = server.directory.split(/[\\/]/).filter(Boolean);
  const folder = parts[parts.length - 1] ?? server.directory;
  return `${folder}/${server.entry || "index.html"}`;
}

export function ServerSettings(): ReactNode {
  const [servers, setServers] = useState<ViteServerInfo[]>([]);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async (): Promise<void> => {
    try {
      setServers(await window.vela.viteServers());
    } catch {
      setServers([]);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), REFRESH_MS);
    return () => clearInterval(timer);
  }, [refresh]);

  const stop = (serverID?: string): void => {
    if (busy) return;
    setBusy(true);
    const request = serverID ? window.vela.viteStop(serverID) : window.vela.viteStopAll();
    void request.then(() => refresh()).catch(() => {}).finally(() => setBusy(false));
  };

  return (
    <section className="settings-section" aria-label="Active servers">
      <div className="settings-callout">
        <strong>{servers.length === 0 ? "No servers running" : `${servers.length} server${servers.length === 1 ? "" : "s"} running`}</strong>
        <p>Vite previews run on loopback ports while Vela is open and stop when you quit. Stop one here to release its port immediately.</p>
      </div>
      <div className="settings-list">
        {servers.length === 0
          ? <div className="settings-empty">Start a preview from the server button in the terminal tray.</div>
          : servers.map((server) => (
            <div className="settings-list-row" key={server.id}>
              <div>
                <strong>{serverLabel(server)}</strong>
                <small>{server.url} · {server.directory}</small>
              </div>
              <button className="settings-action-button" disabled={busy} onClick={() => stop(server.id)}>Stop</button>
            </div>
          ))}
      </div>
      {servers.length > 1 && <button className="btn btn-danger" disabled={busy} onClick={() => stop()}>Stop all servers</button>}
    </section>
  );
}
