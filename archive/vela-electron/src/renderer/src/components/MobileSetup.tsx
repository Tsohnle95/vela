import { useCallback, useEffect, useState, type ReactNode } from "react";
import QRCode from "qrcode";
import type { MobilePairingQr, MobileServerSetupStatus } from "@shared/types";

interface RenderedPairingQr extends MobilePairingQr {
  dataUrl: string;
}

const statusLabel = (status: MobileServerSetupStatus | null, loading: boolean): string => {
  if (loading) return "Checking mobile server…";
  if (!status) return "Mobile server status unavailable";
  switch (status.state) {
    case "ready": return "Ready to pair";
    case "starting": return "Starting mobile server…";
    case "offline": return "Mobile server is not responding";
    case "setup-required":
      return "Mobile server setup required";
  }
};

const statusDescription = (status: MobileServerSetupStatus | null, loading: boolean): ReactNode => {
  if (loading) return "Checking that the mobile companion server is running on this Mac.";
  if (!status) return "Restart Vela and try checking again.";
  if (status.reason === "companion-checkout-missing") {
    return <>Vela's mobile server files were not found at <code>{status.workspacePath}</code>. Set <code>VELA_MOBILE_HOME</code> to the folder that contains <code>packages/web</code>, then restart Vela.</>;
  }
  if (status.state === "starting") return "Vela is starting the mobile server. This panel will update when it is ready.";
  if (status.state === "offline") {
    return <>Keep Vela open and restart it if needed. Logs: <code>~/Library/Application Support/VelaMobile/logs/mobile-server.err.log</code>.</>;
  }
  return "The desktop companion server is running. Generate a one-time code for your phone.";
};

export function MobileSetup(): ReactNode {
  const [status, setStatus] = useState<MobileServerSetupStatus | null>(null);
  const [statusLoading, setStatusLoading] = useState(true);
  const [statusError, setStatusError] = useState("");
  const [pairing, setPairing] = useState<RenderedPairingQr | null>(null);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState("");
  const [clockMs, setClockMs] = useState(() => Date.now());

  const refreshStatus = useCallback(async () => {
    setStatusError("");
    try {
      setStatus(await window.vela.mobileSetupStatus());
    } catch (cause) {
      setStatus(null);
      setStatusError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setStatusLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    if (status?.state !== "starting") return;
    const timer = window.setInterval(() => void refreshStatus(), 1_500);
    return () => window.clearInterval(timer);
  }, [refreshStatus, status?.state]);

  useEffect(() => {
    if (!pairing) return;
    const timer = window.setInterval(() => setClockMs(Date.now()), 1_000);
    return () => window.clearInterval(timer);
  }, [pairing]);

  const createPairing = useCallback(async () => {
    if (creating || status?.state !== "ready") return;
    setCreating(true);
    setError("");
    try {
      const result = await window.vela.mobilePairingQr();
      const dataUrl = await QRCode.toDataURL(result.connectionUrl, {
        width: 480,
        margin: 2,
        errorCorrectionLevel: "L",
      });
      setPairing({ ...result, dataUrl });
      await refreshStatus();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
      await refreshStatus();
    } finally {
      setCreating(false);
    }
  }, [creating, refreshStatus, status?.state]);

  const ready = status?.state === "ready" && !statusLoading;
  const expiresAt = pairing ? new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" }).format(new Date(pairing.expiresAt)) : "";
  const pairingExpired = pairing ? Date.parse(pairing.expiresAt) <= clockMs : false;

  return (
    <section className="settings-section mobile-setup" aria-label="Mobile setup walkthrough">
      <div className="mobile-setup-guide">
        <div className="mobile-setup-intro">
          <p className="mobile-setup-kicker">VELA · PHONE COMPANION</p>
          <h2>Continue your workspace on your phone.</h2>
          <p>Vela Desktop starts an authenticated companion server while this app is open. Pair once; your phone then remembers this desktop. When the desktop OpenCode service is available, conversations are shared across both apps.</p>
        </div>

        <div className="mobile-setup-steps" aria-label="Pairing steps">
          <div className="mobile-setup-step">
            <span className="mobile-setup-number">01</span>
            <div>
              <strong>Prepare this Mac</strong>
              <p>Leave Vela open. This desktop starts the mobile server automatically and stops it when you quit.</p>
            </div>
          </div>
          <div className="mobile-setup-step">
            <span className="mobile-setup-number">02</span>
            <div>
              <strong>Open the mobile companion app</strong>
              <p>On your phone, open the mobile companion app and choose <b>Scan QR code</b> on its connection screen.</p>
            </div>
          </div>
          <div className="mobile-setup-step">
            <span className="mobile-setup-number">03</span>
            <div>
              <strong>Scan the code from this page</strong>
              <p>The code works once and expires after 10 minutes. It contains no reusable password or device token.</p>
            </div>
          </div>
        </div>

        <p className="mobile-setup-network">The code includes this Mac’s reachable network address and a relay route when available. For manual setup, use an address the phone can reach—never <code>localhost</code>.</p>
      </div>

      <div className="mobile-setup-pairing">
        <div className="mobile-setup-pairing-heading">
          <div>
            <p className="mobile-setup-kicker">PAIR A DEVICE</p>
            <h3>Connect the mobile companion</h3>
          </div>
          <span className={`mobile-setup-status ${status?.state === "ready" ? "ready" : status?.state === "setup-required" ? "error" : ""}`} data-state={status?.state ?? "unknown"}>
            <i aria-hidden="true" />
            {statusLabel(status, statusLoading)}
          </span>
        </div>

        {pairing ? (
          <div className="mobile-setup-qr-result">
            {pairingExpired ? <p className="mobile-setup-expired" role="status">This code expired. Generate a new one to pair your phone.</p> : null}
            <img src={pairing.dataUrl} alt={`One-time mobile companion pairing QR for ${pairing.serverLabel}`} className={pairingExpired ? "expired" : ""} />
            <p>{pairingExpired ? "Expired at" : "Scan with the mobile companion before"} <strong>{expiresAt}</strong>.</p>
            <small>One use · expires in 10 minutes</small>
          </div>
        ) : (
          <div className="mobile-setup-qr-placeholder" aria-hidden="true">
            <span className="mobile-setup-qr-reticle"><i /><i /><i /><i /></span>
            <p>Your pairing code will appear here.</p>
          </div>
        )}

        {statusError ? <p className="mobile-setup-error" role="alert">{statusError}</p> : null}
        {error ? <p className="mobile-setup-error" role="alert">{error}</p> : null}
        <p className="mobile-setup-status-copy" aria-live="polite">{statusDescription(status, statusLoading)}</p>
        {status?.state === "offline" ? <p className="mobile-setup-path">Companion folder: <code>{status.workspacePath}</code></p> : null}
        <div className="mobile-setup-actions">
          <button className="mobile-setup-generate" disabled={!ready || creating} onClick={() => void createPairing()}>
            {creating ? "Creating secure code…" : pairing ? "Generate a new pairing QR" : "Generate pairing QR"}
          </button>
          {status?.state !== "ready" && !statusLoading ? (
            <button className="mobile-setup-refresh" onClick={() => { setStatusLoading(true); void refreshStatus(); }}>Check again</button>
          ) : null}
        </div>
      </div>
    </section>
  );
}
