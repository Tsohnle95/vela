import { useRef, useState, type ReactNode } from "react";
import { IconArrowLeft, IconArrowRight, IconClose, IconGlobe, IconRefresh } from "./icons";
import { isBrowserTarget, useBrowserGuest } from "../use-browser-guest";
import { browserTabLabel, type BrowserTab } from "../use-browser-tabs";

// "Search or enter a URL": explicit schemes pass through, anything that looks
// like a host gets https, everything else becomes a search.
export function browserAddress(input: string): string | null {
  const value = input.trim();
  if (value.length === 0) return null;
  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(value) || /^(?:about|file|javascript|data):/i.test(value)) return isBrowserTarget(value) ? value : null;
  if (value.startsWith("localhost") || /^[\w-]+(\.[\w-]+)+(:\d+)?([/?#].*)?$/i.test(value)) {
    return `https://${value}`;
  }
  return `https://duckduckgo.com/?q=${encodeURIComponent(value)}`;
}

function BrowserToolbar({ tab, address, onAddress, onFocusChange, onSubmit, guest }: {
  tab: BrowserTab;
  address: string;
  onAddress: (value: string) => void;
  onFocusChange: (focused: boolean) => void;
  onSubmit: () => void;
  guest: { goBack: () => void; goForward: () => void; reload: () => void; stop: () => void };
}): ReactNode {
  return (
    <div className="browser-bar">
      <button type="button" className="browser-nav" aria-label="Back" disabled={!tab.canGoBack} onClick={guest.goBack}>
        <IconArrowLeft />
      </button>
      <button type="button" className="browser-nav" aria-label="Forward" disabled={!tab.canGoForward} onClick={guest.goForward}>
        <IconArrowRight />
      </button>
      <button
        type="button"
        className="browser-nav"
        aria-label={tab.loading ? "Stop loading" : "Reload"}
        onClick={tab.loading ? guest.stop : guest.reload}
      >
        {tab.loading ? <IconClose /> : <IconRefresh />}
      </button>
      <form
        className="browser-address"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
          (event.currentTarget.querySelector("input") as HTMLInputElement | null)?.blur();
        }}
      >
        <input
          aria-label="Address"
          placeholder="Search or enter a URL"
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          value={address}
          onFocus={() => onFocusChange(true)}
          onBlur={() => onFocusChange(false)}
          onChange={(event) => onAddress(event.target.value)}
          onKeyDown={(event) => {
            if (event.key !== "Escape") return;
            onFocusChange(false);
            (event.target as HTMLInputElement).blur();
          }}
        />
      </form>
    </div>
  );
}

export function BrowserPane({ tab, active, onState }: {
  tab: BrowserTab;
  active: boolean;
  onState: (patch: Partial<Omit<BrowserTab, "id">>) => void;
}): ReactNode {
  const [address, setAddress] = useState(tab.url);
  const editingRef = useRef(false);
  const committedURLRef = useRef(tab.url);
  // While the field has focus the reader's text wins; committed URLs only
  // replace it once they stop editing.
  const guest = useBrowserGuest(onState, (url) => {
    committedURLRef.current = url;
    if (!editingRef.current) setAddress(url);
  }, tab.url);

  const submit = (): void => {
    const target = browserAddress(address);
    if (!target) {
      if (address.trim()) onState({ loading: false, error: "Use a credential-free HTTP or HTTPS address." });
      return;
    }
    setAddress(target);
    guest.navigate(target);
  };

  return (
    <section
      className={`browser-pane${active ? " active" : ""}`}
      aria-label={tab.title || "Browser tab"}
      data-browser-tab={tab.id}
    >
      <BrowserToolbar
        tab={tab}
        address={address}
        onAddress={setAddress}
        onFocusChange={(focused) => {
          editingRef.current = focused;
          if (!focused) setAddress(committedURLRef.current);
        }}
        onSubmit={submit}
        guest={guest}
      />
      <div className="browser-frame-host" ref={guest.hostRef}>
        {tab.url.length === 0 && (
          <div className="browser-placeholder">
            <IconGlobe />
            <p>Search or enter a URL to browse the web beside your files.</p>
          </div>
        )}
        {tab.url.length > 0 && tab.loading && !tab.error && (
          <div className="browser-placeholder loading" role="status">
            <span className="browser-spinner" aria-hidden="true" />
            <p>Loading {browserTabLabel(tab)}…</p>
          </div>
        )}
        {tab.error && (
          <div className="browser-placeholder error" role="status">
            <IconGlobe />
            <p>{tab.error}</p>
            <button type="button" className="toolbar-btn" onClick={guest.reload}>Try again</button>
          </div>
        )}
      </div>
    </section>
  );
}

// All browser tabs stay mounted so their guests keep their state; only the
// active one is laid out.
export function BrowserStack({ tabs, activeID, onState }: {
  tabs: BrowserTab[];
  activeID: string | null;
  onState: (id: string, patch: Partial<Omit<BrowserTab, "id">>) => void;
}): ReactNode {
  if (tabs.length === 0) return null;
  return (
    <div className={`browser-stack${activeID !== null ? " active" : ""}`}>
      {tabs.map((tab) => (
        <BrowserPane
          key={tab.id}
          tab={tab}
          active={tab.id === activeID}
          onState={(patch) => onState(tab.id, patch)}
        />
      ))}
    </div>
  );
}
