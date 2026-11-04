import { useCallback, useEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import type { ModelOption, ProviderUsageCredits, ProviderUsageResult, SessionUsage } from "@shared/types";
import { IconDashboard, IconRefresh } from "./icons";
import { useMenuDismiss } from "./use-menu-dismiss";

function formatTokens(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(count >= 10_000_000 ? 0 : 1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(count >= 100_000 ? 0 : 1)}k`;
  return String(count);
}

function formatCost(cost: number): string {
  if (cost >= 1) return `$${cost.toFixed(2)}`;
  if (cost > 0) return `$${cost.toFixed(4)}`;
  return "$0.00";
}

function formatCount(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return String(value);
}

function formatPlan(plan: string): string {
  return plan
    .replace(/[_-]+/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase())
    .replace("Self Serve Business", "Self-serve Business");
}

function formatResets(resetsAt: number): string {
  const remainingMs = resetsAt * 1000 - Date.now();
  if (remainingMs <= 0) return "resets any moment";
  const minutes = Math.ceil(remainingMs / 60_000);
  if (minutes < 60) return `resets in ${minutes}m`;
  const hours = Math.floor(minutes / 60);
  if (hours >= 48) {
    const days = Math.floor(hours / 24);
    const remHours = hours % 24;
    return remHours > 0 ? `resets in ${days}d ${remHours}h` : `resets in ${days}d`;
  }
  const rem = minutes % 60;
  return rem > 0 ? `resets in ${hours}h ${rem}m` : `resets in ${hours}h`;
}

function windowTone(percent: number): "ok" | "warn" | "danger" {
  if (percent >= 90) return "danger";
  if (percent >= 60) return "warn";
  return "ok";
}

function contextTone(percent: number): "ok" | "warn" | "danger" {
  if (percent >= 85) return "danger";
  if (percent >= 60) return "warn";
  return "ok";
}

function formatCredits(credits: ProviderUsageCredits): string {
  if (credits.unlimited) return "Unlimited";
  if (credits.used != null && credits.total != null) {
    return `${formatCount(credits.used)} / ${formatCount(credits.total)}`;
  }
  if (credits.remaining != null) return formatCount(credits.remaining);
  return credits.balance ?? "";
}

function formatTurnElapsed(elapsedMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}h ${String(minutes).padStart(2, "0")}m`;
  if (minutes > 0) return `${minutes}m ${String(seconds).padStart(2, "0")}s`;
  return `${seconds}s`;
}

export function TurnTimer({ startedAt }: { startedAt: number | null }): ReactNode {
  const [, setTick] = useState(0);
  useEffect(() => {
    if (startedAt === null) return;
    const timer = setInterval(() => setTick((tick) => tick + 1), 1000);
    return () => clearInterval(timer);
  }, [startedAt]);
  if (startedAt === null) return null;
  return (
    <span className="agent-turn-timer" title="Time since the current turn's prompt was sent">
      <span className="agent-turn-timer-dot" />
      {formatTurnElapsed(Date.now() - startedAt)}
    </span>
  );
}

function ProviderUsageCard({ result }: { result: ProviderUsageResult }): ReactNode {
  const snapshot = result.snapshot;
  return (
    <div className="usage-provider">
      <div className="usage-provider-head">
        <span className="usage-provider-name">{result.displayName}</span>
        {snapshot?.planType && <span className="usage-provider-plan">{formatPlan(snapshot.planType)}</span>}
        <span className={`usage-provider-dot ${result.status}`} title={result.error?.message ?? result.status} />
      </div>
      {!snapshot && result.error && <div className="usage-provider-error">{result.error.message}</div>}
      {snapshot?.windows.map((window) => (
        <div className="usage-window" key={window.id}>
          <div className="usage-window-row">
            <span className="usage-window-label">{window.label}</span>
            <span className="usage-window-value">{Math.round(100 - Math.min(100, Math.max(0, window.usedPercent)))}% left</span>
          </div>
          <div className="usage-window-bar">
            <div
              className={`usage-window-fill ${windowTone(window.usedPercent)}`}
              style={{ width: `${Math.min(100, Math.max(0, window.usedPercent))}%` }}
            />
          </div>
          {window.resetsAt && <div className="usage-window-reset">{formatResets(window.resetsAt)}</div>}
        </div>
      ))}
      {snapshot?.credits && (snapshot.credits.hasCredits || snapshot.credits.unlimited || snapshot.credits.used != null || snapshot.credits.remaining != null) && (
        <div className="usage-credits">
          <span className="usage-credits-label">{snapshot.credits.label ?? "Credits"}</span>
          <span className="usage-credits-value">{formatCredits(snapshot.credits)}</span>
        </div>
      )}
      {!snapshot && !result.error && (
        <div className="usage-provider-error">No usage data reported for this provider.</div>
      )}
    </div>
  );
}

interface UsageOptions {
  sessionUsage: SessionUsage | null;
  compactionBaseline: number | null;
  currentModel: ModelOption | null;
  providerUsage: ProviderUsageResult[];
  providerUsageLoading: boolean;
  refreshProviderUsage: () => Promise<void>;
  panelRef: RefObject<HTMLDivElement | null>;
  onOpen: () => void;
}
export function useAgentUsage({ sessionUsage, compactionBaseline, currentModel, providerUsage,
  providerUsageLoading, refreshProviderUsage, panelRef, onOpen }: UsageOptions) {
  const [usageOpen, setUsageOpen] = useState(false);
  const [providerUsageRefreshing, setProviderUsageRefreshing] = useState(false);
  const [providerUsageRefreshKey, setProviderUsageRefreshKey] = useState(0);
  const providerUsageLoadingRef = useRef(providerUsageLoading);
  const providerUsageRequestActiveRef = useRef(false);
  providerUsageLoadingRef.current = providerUsageLoading;
  const triggerProviderUsageRefresh = useCallback((): void => {
    if (providerUsageLoadingRef.current || providerUsageRequestActiveRef.current) return;
    providerUsageRequestActiveRef.current = true;
    setProviderUsageRefreshing(true);
    setProviderUsageRefreshKey((current) => current + 1);
    void Promise.resolve(refreshProviderUsage()).finally(() => {
      providerUsageRequestActiveRef.current = false;
      setProviderUsageRefreshing(false);
    });
  }, [refreshProviderUsage]);

  useEffect(() => {
    if (usageOpen) triggerProviderUsageRefresh();
  }, [usageOpen, triggerProviderUsageRefresh]);

  useMenuDismiss(usageOpen, panelRef, useCallback(() => setUsageOpen(false), []));

  const usage = sessionUsage;
  const usageTotal = usage
    ? usage.tokens.input + usage.tokens.output + usage.tokens.reasoning + usage.tokens.cache.read + usage.tokens.cache.write
    : 0;
  const cacheTotal = usage ? usage.tokens.cache.read + usage.tokens.cache.write : 0;
  const cachePercent = usageTotal > 0 ? (cacheTotal / usageTotal) * 100 : 0;
  const usageShare = (count: number): string =>
    usageTotal > 0 ? `${(count / usageTotal) * 100}%` : "0%";
  const contextLimit =
    typeof currentModel?.limit?.context === "number" && Number.isFinite(currentModel.limit.context) && currentModel.limit.context > 0
      ? currentModel.limit.context
      : null;
  const rawContextUsed = usage ? usage.tokens.input : 0;
  const contextUsed = usage && compactionBaseline !== null && compactionBaseline !== undefined
    ? Math.max(0, rawContextUsed - compactionBaseline)
    : rawContextUsed;
  const contextFraction = contextLimit ? Math.min(1, Math.max(0, contextUsed / contextLimit)) : 0;
  const contextPercent = contextFraction * 100;
  const glyphTone = contextLimit ? contextTone(contextPercent) : null;
  const usageButton = (
    <button
      role="menuitem"
      className={`agent-usage-toggle ${usageOpen ? "open" : ""} ${glyphTone ?? "neutral"}`}
      title="Session and provider usage"
      aria-label={contextLimit ? `Context usage ${Math.round(contextPercent)} percent` : "Context and usage"}
      aria-expanded={usageOpen}
      onClick={() => { onOpen(); setUsageOpen((open) => !open); }}
    >
      <span className="agent-usage-menu-label">
        <span className="agent-usage-ring" aria-hidden="true">
          <svg viewBox="0 0 20 20">
            <circle className="agent-usage-ring-track" cx="10" cy="10" r="8" />
            <circle
              className="agent-usage-ring-fill"
              cx="10"
              cy="10"
              r="8"
              style={{ strokeDashoffset: `${50.27 * (1 - (contextLimit ? contextPercent : 0) / 100)}` }}
            />
          </svg>
        </span>
        <span>Context</span>
      </span>
      <small>{contextLimit ? `${Math.round(contextPercent)}% used · session and provider usage` : "Session and provider usage"}</small>
    </button>
  );

  const usagePopup = usageOpen ? <AgentUsagePopup {...{ usage, compactionBaseline, usageTotal, cacheTotal, cachePercent,
    usageShare, contextLimit, rawContextUsed, contextUsed, contextPercent, providerUsage, providerUsageLoading,
    providerUsageRefreshing, providerUsageRefreshKey, triggerProviderUsageRefresh }} /> : null;
  return { usageButton, usagePopup, closeUsage: () => setUsageOpen(false) };
}

function AgentUsagePopup({ usage, compactionBaseline, usageTotal, cacheTotal, cachePercent, usageShare,
  contextLimit, rawContextUsed, contextUsed, contextPercent, providerUsage, providerUsageLoading,
  providerUsageRefreshing, providerUsageRefreshKey, triggerProviderUsageRefresh }: {
  usage: SessionUsage | null;
  compactionBaseline: number | null;
  usageTotal: number;
  cacheTotal: number;
  cachePercent: number;
  usageShare: (count: number) => string;
  contextLimit: number | null;
  rawContextUsed: number;
  contextUsed: number;
  contextPercent: number;
  providerUsage: ProviderUsageResult[];
  providerUsageLoading: boolean;
  providerUsageRefreshing: boolean;
  providerUsageRefreshKey: number;
  triggerProviderUsageRefresh: () => void;
}): ReactNode {
  return (

        <div className="agent-usage-popup">
          <div className="agent-usage-scroll">
            <div className="agent-usage-head">
              <IconDashboard />
              Session usage
            </div>
            {usage ? (
              <>
                <div className="agent-usage-cost">
                  <span className="agent-usage-cost-label">Total cost</span>
                  <span className="agent-usage-cost-value">{formatCost(usage.cost)}</span>
                </div>
                {contextLimit !== null && (
                  <div className="agent-usage-context">
                    <div className="agent-usage-context-head">
                      <span className="agent-usage-context-label">Context window</span>
                      <span className={`agent-usage-context-percent ${contextTone(contextPercent)}`}>
                        {Math.round(contextPercent)}%
                      </span>
                    </div>
                    <div className="agent-usage-context-bar">
                      <div
                        className={`agent-usage-context-fill ${contextTone(contextPercent)}`}
                        style={{ width: `${Math.min(100, Math.max(0, contextPercent))}%` }}
                      />
                    </div>
                    <div className="agent-usage-context-counts">
                      <span>{formatTokens(contextUsed)}</span>
                      <span>of {formatTokens(contextLimit)} tokens</span>
                    </div>
                    {compactionBaseline !== null && compactionBaseline > 0 && (
                      <div className="agent-usage-context-note" style={{ fontSize: "11px", color: "var(--v2-text-text-muted)", marginTop: "4px" }}>
                        {formatTokens(rawContextUsed)} total · {formatTokens(compactionBaseline)} before last compaction
                      </div>
                    )}
                  </div>
                )}
                {usageTotal > 0 && (
                  <div className="agent-usage-bar">
                    <span className="agent-usage-seg input" style={{ width: usageShare(usage.tokens.input) }} />
                    <span className="agent-usage-seg output" style={{ width: usageShare(usage.tokens.output) }} />
                    <span className="agent-usage-seg reasoning" style={{ width: usageShare(usage.tokens.reasoning) }} />
                    <span className="agent-usage-seg cache" style={{ width: usageShare(usage.tokens.cache.read) }} />
                  </div>
                )}
                <div className="agent-usage-rows">
                  {[
                    { label: "Input", count: usage.tokens.input }, { label: "Output", count: usage.tokens.output },
                    { label: "Reasoning", count: usage.tokens.reasoning }, { label: "Cache read", count: usage.tokens.cache.read },
                    { label: "Cache write", count: usage.tokens.cache.write }
                  ].map(({ label, count }) => (
                    <div className="agent-usage-row" key={label}>
                      <span className="agent-usage-row-label">{label}</span>
                      <span className="agent-usage-row-value">{formatTokens(count)}</span>
                    </div>
                  ))}
                  <div className="agent-usage-row agent-usage-cache-total">
                    <span className="agent-usage-row-label">Cache</span>
                    <span className="agent-usage-row-value agent-usage-cache-value">
                      {cacheTotal > 0 ? `${formatTokens(cacheTotal)} · ${Math.round(cachePercent)}%` : formatTokens(cacheTotal)}
                    </span>
                  </div>
                </div>
                <div className="agent-usage-total">
                  <span className="agent-usage-total-label">Total tokens</span>
                  <span className="agent-usage-total-value">{formatTokens(usageTotal)}</span>
                </div>
              </>
            ) : (
              <div className="agent-usage-empty">No usage recorded for this session yet.</div>
            )}
            <ProviderUsageSection {...{ providerUsage, providerUsageLoading, providerUsageRefreshing,
              providerUsageRefreshKey, triggerProviderUsageRefresh }} />
          </div>
        </div>
  );
}

function ProviderUsageSection({ providerUsage, providerUsageLoading, providerUsageRefreshing,
  providerUsageRefreshKey, triggerProviderUsageRefresh }: {
  providerUsage: ProviderUsageResult[];
  providerUsageLoading: boolean;
  providerUsageRefreshing: boolean;
  providerUsageRefreshKey: number;
  triggerProviderUsageRefresh: () => void;
}): ReactNode {
  return (
            <div className="usage-provider-section">
              <div className="usage-provider-head">
                <span className="usage-provider-title">
                  <svg className="agent-usage-glyph" viewBox="0 0 16 16" aria-hidden="true">
                    <circle cx="8" cy="8" r="6.4" fill="none" stroke="currentColor" strokeWidth="1.6" />
                    <circle cx="8" cy="8" r="2.1" fill="currentColor" />
                  </svg>
                  Provider usage
                </span>
                <button
                  type="button"
                  className="usage-provider-refresh"
                  title="Refresh provider usage"
                  aria-label="Refresh provider usage"
                  disabled={providerUsageLoading || providerUsageRefreshing}
                  onClick={triggerProviderUsageRefresh}
                >
                  <IconRefresh key={providerUsageRefreshKey} className={providerUsageRefreshKey > 0 ? "spinning" : ""} />
                </button>
              </div>
              {providerUsageLoading && providerUsage.length === 0 ? (
                <div className="usage-provider-loading">
                  <span className="spinner" />
                  Checking provider usage…
                </div>
              ) : providerUsage.length === 0 ? (
                <div className="usage-provider-empty">
                  No providers connected. Sign in with <code>opencode auth login</code> to see plan limits here.
                </div>
              ) : (
                providerUsage.map((result) => <ProviderUsageCard key={result.provider} result={result} />)
              )}
            </div>
  );
}
