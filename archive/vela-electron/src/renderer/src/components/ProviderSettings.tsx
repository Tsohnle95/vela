import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import type {
  ProviderCredentialAnswers,
  ProviderCredentialValue,
  ProviderFormField,
  ProviderIntegration,
  ProviderOAuthAttempt,
  ProviderUsageResult,
  WorkspaceIdentity
} from "@shared/types";
import { ExternalLink } from "./ExternalLink";

function initialAnswers(fields: ProviderFormField[]): ProviderCredentialAnswers {
  return Object.fromEntries(fields.flatMap((field) => field.default === undefined ? [] : [[field.key, field.default]]));
}

function fieldVisible(field: ProviderFormField, answers: ProviderCredentialAnswers): boolean {
  return (field.when ?? []).every((condition) => condition.op === "eq"
    ? answers[condition.key] === condition.value
    : answers[condition.key] !== condition.value);
}

function fieldControl(
  field: ProviderFormField,
  value: ProviderCredentialValue | undefined,
  setValue: (value: ProviderCredentialValue) => void
): ReactNode {
  if (field.type === "external") {
    return field.url ? <ExternalLink className="provider-external" href={field.url}>Open setup instructions</ExternalLink> : null;
  }
  if (field.type === "boolean") {
    return <input type="checkbox" checked={value === true} onChange={(event) => setValue(event.target.checked)} />;
  }
  if (field.type === "multiselect") {
    return (
      <select multiple value={Array.isArray(value) ? value : []} onChange={(event) => setValue([...event.target.selectedOptions].map((option) => option.value))}>
        {(field.options ?? []).map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
      </select>
    );
  }
  if (field.options) {
    return (
      <select value={typeof value === "string" ? value : ""} onChange={(event) => setValue(event.target.value)} required={field.required}>
        <option value="">Select</option>
        {field.options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
      </select>
    );
  }
  return (
    <input
      type={field.type === "number" || field.type === "integer" ? "number" : "text"}
      spellCheck={false}
      autoCorrect="off"
      autoCapitalize="off"
      step={field.type === "integer" ? 1 : undefined}
      min={field.minimum}
      max={field.maximum}
      minLength={field.minLength}
      maxLength={field.maxLength}
      pattern={field.pattern}
      placeholder={field.placeholder}
      value={typeof value === "string" || typeof value === "number" ? value : ""}
      required={field.required}
      onChange={(event) => setValue(field.type === "number" || field.type === "integer"
        ? (event.target.value === "" ? "" : event.target.valueAsNumber)
        : event.target.value)}
    />
  );
}

function OAuthFlow({
  provider,
  workspace,
  refresh,
  refreshModels,
  onFinished
}: {
  provider: ProviderIntegration;
  workspace: WorkspaceIdentity;
  refresh: () => Promise<void>;
  refreshModels: () => Promise<void>;
  onFinished: () => void;
}): ReactNode {
  const [method, setMethod] = useState<{ id: string; label: string } | null>(provider.oauth[0] ?? null);
  const [attempt, setAttempt] = useState<ProviderOAuthAttempt | null>(null);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const finishedRef = useRef(false);

  useEffect(() => {
    if (!attempt || attempt.mode !== "auto") return;
    let timer: ReturnType<typeof setInterval> | null = null;
    const tick = async (): Promise<void> => {
      try {
        const poll = await window.vela.providerOauthPoll(workspace, provider.id, attempt.attemptID);
        if (poll.status === "complete") {
          finishedRef.current = true;
          if (timer) clearInterval(timer);
          await Promise.all([refresh(), refreshModels()]);
          onFinished();
        } else if (poll.status === "expired" || poll.status === "failed") {
          if (timer) clearInterval(timer);
          setError(poll.status === "failed" ? poll.message : "Authorization request expired");
          setAttempt(null);
        }
      } catch {
        if (timer) clearInterval(timer);
        setError("Lost contact with the authorization service");
        setAttempt(null);
      }
    };
    void tick();
    timer = setInterval(() => void tick(), 2000);
    return () => {
      if (timer) clearInterval(timer);
    };
  }, [attempt, provider.id, workspace, refresh, refreshModels, onFinished]);

  useEffect(() => () => {
    if (attempt && !finishedRef.current) {
      void window.vela.providerOauthCancel(workspace, provider.id, attempt.attemptID).catch(() => {});
    }
  }, [attempt, provider.id, workspace]);

  const start = async (): Promise<void> => {
    if (!method) return;
    setBusy(true);
    setError(null);
    finishedRef.current = false;
    try {
      setAttempt(await window.vela.providerOauthStart(workspace, provider.id, method.id));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };

  const confirmCode = async (): Promise<void> => {
    if (!attempt || !code) return;
    setBusy(true);
    try {
      await window.vela.providerOauthComplete(workspace, provider.id, attempt.attemptID, code);
      await Promise.all([refresh(), refreshModels()]);
      onFinished();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="provider-oauth">
      {!attempt ? (
        <button className="primary" disabled={busy || !method} onClick={() => void start()}>
          {busy ? "Starting..." : method ? `Connect with ${method.label}` : "Connect"}
        </button>
      ) : (
        <>
          <p className="provider-note">{attempt.instructions || "Follow the authorization prompt"}</p>
          {attempt.url && <ExternalLink className="provider-external" href={attempt.url}>Open authorization page</ExternalLink>}
          {attempt.mode === "code" && (
            <label><span>Authorization code</span>
              <input value={code} spellCheck={false} autoCorrect="off" autoCapitalize="off" onChange={(event) => setCode(event.target.value)} placeholder="Paste the code" autoFocus />
            </label>
          )}
          <div className="provider-form-actions">
            {attempt.mode === "code"
              ? <button className="primary" disabled={busy || !code} onClick={() => void confirmCode()}>{busy ? "Confirming..." : "Confirm code"}</button>
              : <span className="provider-note">Waiting for authorization…</span>}
            <button onClick={() => { setAttempt(null); setCode(""); }}>Cancel</button>
          </div>
        </>
      )}
      {provider.oauth.length > 1 && !attempt && (
        <select value={method?.id ?? ""} onChange={(event) => setMethod(provider.oauth.find((option) => option.id === event.target.value) ?? null)}>
          {provider.oauth.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
        </select>
      )}
      {error && <p className="provider-error">{error}</p>}
    </div>
  );
}

function ProviderCard({
  provider,
  usage,
  workspace,
  refresh,
  refreshModels
}: {
  provider: ProviderIntegration;
  usage?: ProviderUsageResult;
  workspace: WorkspaceIdentity;
  refresh: () => Promise<void>;
  refreshModels: () => Promise<void>;
}): ReactNode {
  const [open, setOpen] = useState(false);
  const [key, setKey] = useState("");
  const [label, setLabel] = useState("");
  const [answers, setAnswers] = useState<ProviderCredentialAnswers>(() => initialAnswers(provider.keyMethod?.fields ?? []));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const connected = provider.credentials.length > 0 || provider.environment.connected.length > 0;
  const displayName = provider.name;

  const submit = async (event: FormEvent): Promise<void> => {
    event.preventDefault();
    if (!provider.keyMethod || !key) return;
    setSaving(true);
    setError(null);
    try {
      const visibleKeys = new Set(provider.keyMethod.fields.filter((field) => field.type !== "external" && fieldVisible(field, answers)).map((field) => field.key));
      const submittedAnswers = Object.fromEntries(Object.entries(answers).filter(([answerKey, value]) => visibleKeys.has(answerKey) && value !== "" && (!Array.isArray(value) || value.length > 0)));
      await window.vela.connectProviderKey(workspace, provider.id, key, label, submittedAnswers);
      setKey("");
      setLabel("");
      setOpen(false);
      await Promise.all([refresh(), refreshModels()]);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  const remove = async (credentialID: string): Promise<void> => {
    if (!window.confirm(`Remove this ${displayName} credential?`)) return;
    setSaving(true);
    setError(null);
    try {
      await window.vela.removeProviderCredential(workspace, credentialID);
      await Promise.all([refresh(), refreshModels()]);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSaving(false);
    }
  };

  return (
    <article className={`provider-card ${connected ? "connected" : ""}`}>
      <div className="provider-card-heading">
        <span className="provider-monogram">{displayName.slice(0, 2).toUpperCase()}</span>
        <div><strong>{displayName}</strong><small>{provider.id}</small></div>
        <span className={`settings-badge ${connected ? "available" : ""}`}>{connected ? "Connected" : "Available"}</span>
      </div>

      {provider.credentials.length > 0 && <div className="provider-connections">
        {provider.credentials.map((credential) => (
          <div key={credential.id}><span><i />{credential.label}</span><button disabled={saving} onClick={() => void remove(credential.id)}>Remove</button></div>
        ))}
      </div>}

      {provider.environment.connected.length > 0 && <p className="provider-note">Connected from {provider.environment.connected.join(", ")}</p>}
      {usage?.snapshot?.planType && <p className="provider-note">Plan: {usage.snapshot.planType}</p>}

      {open && provider.keyMethod ? (
        <form className="provider-key-form" onSubmit={(event) => void submit(event)}>
          <label><span>{provider.keyMethod.label ?? "API key"}</span><input type="password" autoComplete="off" spellCheck={false} autoCorrect="off" autoCapitalize="off" value={key} onChange={(event) => setKey(event.target.value)} required autoFocus /></label>
          <label><span>Label <small>optional</small></span><input value={label} spellCheck={false} autoCorrect="off" autoCapitalize="off" onChange={(event) => setLabel(event.target.value)} placeholder="Work, personal, team..." /></label>
          {provider.keyMethod.fields.filter((field) => fieldVisible(field, answers)).map((field) => (
            <label key={field.key}>
              <span>{field.title ?? field.key}{field.required ? " *" : ""}</span>
              {fieldControl(field, answers[field.key], (value) => setAnswers((current) => ({ ...current, [field.key]: value })))}
              {field.description && <small>{field.description}</small>}
            </label>
          ))}
          {error && <p className="provider-error">{error}</p>}
          <div className="provider-form-actions"><button type="button" onClick={() => { setKey(""); setLabel(""); setAnswers(initialAnswers(provider.keyMethod?.fields ?? [])); setOpen(false); }}>Cancel</button><button className="primary" disabled={saving}>{saving ? "Connecting..." : "Connect provider"}</button></div>
        </form>
      ) : open && provider.oauth.length > 0 ? (
        <OAuthFlow
          provider={provider}
          workspace={workspace}
          refresh={refresh}
          refreshModels={refreshModels}
          onFinished={() => { setOpen(false); }}
        />
      ) : (
        <div className="provider-card-footer">
          <span>{provider.environment.names.length > 0 ? `Also supports ${provider.environment.names.join(", ")}` : provider.oauth.length > 0 ? "Sign-in available through the provider" : "API key connection"}</span>
          {provider.keyMethod && <button onClick={() => { setKey(""); setLabel(""); setAnswers(initialAnswers(provider.keyMethod?.fields ?? [])); setOpen(true); }}>Add key</button>}
          {provider.oauth.length > 0 && !provider.keyMethod && <button onClick={() => setOpen(true)}>Sign in</button>}
        </div>
      )}
      {!open && error && <p className="provider-error">{error}</p>}
    </article>
  );
}

export function ProviderSettings({
  workspace,
  usage,
  refreshModels
}: {
  workspace: WorkspaceIdentity | null;
  usage: ProviderUsageResult[];
  refreshModels: () => Promise<void>;
}): ReactNode {
  const [providers, setProviders] = useState<ProviderIntegration[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestRef = useRef(0);

  const refresh = async (): Promise<void> => {
    if (!workspace) return;
    const request = ++requestRef.current;
    setLoading(true);
    setError(null);
    try {
      const next = await window.vela.providerIntegrations(workspace);
      if (request === requestRef.current) setProviders(next);
    } catch (cause) {
      if (request === requestRef.current) setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      if (request === requestRef.current) setLoading(false);
    }
  };

  useEffect(() => {
    requestRef.current += 1;
    setProviders([]);
    setError(null);
    if (!workspace) {
      setLoading(false);
      return;
    }
    void refresh();
  }, [workspace?.id, workspace?.generation]);

  const usageByProvider = new Map(usage.map((item) => [item.provider, item]));

  if (!workspace) return <div className="settings-empty">Open a workspace to connect model providers.</div>;

  return (
    <div className="provider-settings">
      <div className="provider-toolbar">
        <div><strong>Bring your own provider</strong><small>Keys are stored by the active agent runtime and are never displayed again.</small></div>
      </div>
      {error && <div className="settings-callout provider-error"><strong>Providers unavailable</strong><p>{error}</p></div>}
      {loading && providers.length === 0 ? <div className="settings-empty">Loading providers...</div> : <div className="provider-grid">
        {providers.map((provider) => <ProviderCard key={provider.id} provider={provider} usage={usageByProvider.get(provider.id)} workspace={workspace} refresh={refresh} refreshModels={refreshModels} />)}
      </div>}
      {!loading && providers.length === 0 && <div className="settings-empty">No providers are available for this runtime.</div>}
    </div>
  );
}
