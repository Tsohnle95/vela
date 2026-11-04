import { useEffect, useRef, useState, type ReactNode } from "react";
import type { Tab, W3cDiagnostic } from "@shared/types";
import { useStore } from "../store";
import { applyW3cMarkers, clearW3cMarkers } from "../w3c-validation";
import { monaco } from "../monaco";
import { getEditor } from "../reveal";
import type { ValidationReport } from "../validation-report";
import type { EditorStatus } from "./editor-status";
import { useSassWatch } from "../use-sass-watch";

const W3C_FILE = /\.(?:html?|css)$/i;
const SCSS_FILE = /\.scss$/i;

function scssDiagnostics(path: string): W3cDiagnostic[] {
  const model = getEditor(path)?.getModel();
  if (!model || model.isDisposed() || model.getLanguageId() !== "scss") return [];
  return monaco.editor.getModelMarkers({ resource: model.uri })
    .filter((marker) => marker.owner === "scss" && (marker.severity === monaco.MarkerSeverity.Error || marker.severity === monaco.MarkerSeverity.Warning))
    .map((marker) => ({
      line: marker.startLineNumber,
      column: marker.startColumn,
      endLine: marker.endLineNumber,
      endColumn: marker.endColumn,
      message: marker.message,
      severity: marker.severity === monaco.MarkerSeverity.Warning ? "warning" : "error",
      source: "monaco-scss"
    }));
}

function setValidationDecorations(path: string, visible: boolean): void {
  const editor = getEditor(path);
  editor?.updateOptions?.({ renderValidationDecorations: visible ? "on" : "off" });
}

interface ValidateResult {
  errors: number;
  warnings: number;
  failed: boolean;
  diagnostics: W3cDiagnostic[];
}

interface ValidationControl {
  scssFile: boolean;
  validatableFile: boolean;
  running: boolean;
  result: ValidateResult | null;
  markersShown: boolean;
  validate: () => void;
}

function useValidationControl(
  activeTab: Tab | undefined,
  onValidationComplete?: (report: ValidationReport) => void
): ValidationControl {
  const scssFile = activeTab !== undefined && SCSS_FILE.test(activeTab.path);
  const w3cFile = activeTab !== undefined && W3C_FILE.test(activeTab.path);
  const validatableFile = w3cFile || scssFile;
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<ValidateResult | null>(null);
  const [markersShown, setMarkersShown] = useState(false);
  const runIdRef = useRef(0);

  useEffect(() => {
    runIdRef.current += 1;
    if (activeTab?.path) setValidationDecorations(activeTab.path, true);
    setRunning(false);
    setResult(null);
    setMarkersShown(false);
  }, [activeTab?.path, activeTab?.content]);

  useEffect(() => () => {
    if (activeTab?.path) setValidationDecorations(activeTab.path, true);
  }, [activeTab?.path]);

  const toggleMarkers = (path: string): void => {
    if (!result) return;
    const show = !markersShown;
    if (scssFile) setValidationDecorations(path, show);
    else if (show) applyW3cMarkers(path, result.diagnostics);
    else clearW3cMarkers(path);
    setMarkersShown(show);
  };

  const runValidation = (tab: Tab): void => {
    const runId = ++runIdRef.current;
    setRunning(true);
    setResult(null);
    const validation = scssFile
      ? Promise.resolve(scssDiagnostics(tab.path))
      : window.vela.validateW3c(tab.path, tab.content);
    void validation
      .then((diagnostics) => {
        if (runId !== runIdRef.current) return;
        if (scssFile) setValidationDecorations(tab.path, true);
        else applyW3cMarkers(tab.path, diagnostics);
        const errors = diagnostics.filter((d) => d.severity === "error").length;
        setRunning(false);
        setResult({ errors, warnings: diagnostics.length - errors, failed: false, diagnostics });
        setMarkersShown(diagnostics.length > 0);
        onValidationComplete?.({ path: tab.path, diagnostics });
      })
      .catch((error: unknown) => {
        if (runId !== runIdRef.current) return;
        setRunning(false);
        setResult({ errors: 0, warnings: 0, failed: true, diagnostics: [] });
        onValidationComplete?.({
          path: tab.path,
          diagnostics: [],
          error: error instanceof Error ? error.message : String(error)
        });
      });
  };

  const validate = (): void => {
    if (!activeTab || !validatableFile || running) return;
    if (result && !result.failed && result.diagnostics.length > 0) {
      toggleMarkers(activeTab.path);
      return;
    }
    runValidation(activeTab);
  };

  return { scssFile, validatableFile, running, result, markersShown, validate };
}

function canToggleMarkersFor(control: ValidationControl): boolean {
  return Boolean(control.result && !control.result.failed && control.result.diagnostics.length > 0);
}

function resultLabel(result: ValidateResult | null): string | null {
  if (!result) return null;
  if (result.failed) return "Validation failed";
  if (result.errors > 0) {
    const warnings = result.warnings > 0 ? `, ${result.warnings} warning${result.warnings === 1 ? "" : "s"}` : "";
    return `${result.errors} error${result.errors === 1 ? "" : "s"}${warnings}`;
  }
  if (result.warnings > 0) return `${result.warnings} warning${result.warnings === 1 ? "" : "s"}`;
  return "No problems";
}

function resultTone(result: ValidateResult | null): string {
  if (!result) return "";
  if (result.failed || result.errors > 0) return result.failed ? "failed" : "has-errors";
  return result.warnings > 0 ? "has-warnings" : "clean";
}

function validationLabel(control: ValidationControl): string {
  const canToggle = canToggleMarkersFor(control);
  if (control.running) return "Validating…";
  if (canToggle) return control.markersShown ? "Hide errors" : "Show errors";
  return control.result ? "Validate again" : "Validate";
}

function validationTitle(control: ValidationControl): string {
  if (!control.validatableFile) return "Open an HTML, CSS, or SCSS file to run validation";
  if (canToggleMarkersFor(control)) return control.markersShown ? "Hide errors" : "Show errors";
  return control.scssFile
    ? "Show Sass diagnostics from the SCSS language service"
    : "Run the W3C Nu Html Checker / CSS Validator on the open file";
}

function ValidateItem({ control }: { control: ValidationControl }): ReactNode {
  const label = resultLabel(control.result);
  return (
    <button
      className="status-menu-item validate-btn"
      data-testid="validate-btn"
      role="menuitem"
      disabled={!control.validatableFile || control.running}
      title={validationTitle(control)}
      onClick={control.validate}
    >
      {control.running && <span className="validate-spinner" aria-hidden="true" />}
      {validationLabel(control)}
      {label && (
        <span className={`validate-result ${resultTone(control.result)}`} data-testid="validate-result">
          {label}
        </span>
      )}
    </button>
  );
}

function SassWatchItem(): ReactNode {
  const { session } = useStore();
  const sass = useSassWatch(session);
  const label = sass.state?.status === "starting"
    ? "Starting Sass…"
    : sass.state?.status === "error"
      ? "Sass: Error"
      : sass.state?.enabled ? "Sass: On" : "Watch Sass";
  const title = !session
    ? "Open a workspace to watch Sass"
    : !sass.available
      ? "Restart Vela to enable Sass watching"
      : sass.state?.error
        ? `${sass.state.error}\nSee Sass Output in the terminal panel. Click to ${sass.state.enabled ? "stop watching" : "retry starting"}.`
        : `${sass.state?.enabled ? "Stop" : "Start"} Sass watching in ${session.directory}; saved Sass files compile to CSS on disk`;
  return (
    <button
      className={`status-menu-item sass-watch-btn ${sass.state?.status ?? "off"}`}
      role="menuitem"
      aria-pressed={sass.state?.enabled ?? false}
      aria-label={`Watch Sass for ${session?.directory ?? "workspace"}: ${sass.state?.status ?? "off"}`}
      title={title}
      disabled={!session || !sass.available || sass.pending}
      onClick={() => void sass.toggle()}
    >
      {label}
    </button>
  );
}

function StatusMeta({ tab, editorStatus }: { tab: Tab | undefined; editorStatus: EditorStatus | null }): ReactNode {
  const cursor = editorStatus?.path === tab?.path ? editorStatus : null;
  if (!tab) return null;
  return (
    <div className="status-menu-meta">
      <span className="status-menu-item-meta" data-testid="cursor-position">
        Ln {cursor?.lineNumber ?? 1}, Col {cursor?.column ?? 1}
      </span>
      <span className="status-menu-item-meta" data-testid="indentation-status">
        {cursor?.insertSpaces === false ? "Tab Size" : "Spaces"}: {cursor?.tabSize ?? 2}
      </span>
      <span className="status-menu-item-meta" data-testid="encoding-status">UTF-8</span>
      <span className="status-menu-path" title={tab.path}>{tab.path}</span>
    </div>
  );
}

// The reference app has no footer bar; the editor's status read-outs and the
// validate / Sass-watch controls live in this menu behind the "⋯" button that
// sits at the right end of the breadcrumb row.
export function EditorStatusMenu({
  tab,
  editorStatus = null,
  onValidationComplete
}: {
  tab?: Tab;
  editorStatus?: EditorStatus | null;
  onValidationComplete?: (report: ValidationReport) => void;
}): ReactNode {
  const { tabs, activePath } = useStore();
  const activeTab = tab ?? tabs.find((candidate) => candidate.path === activePath);
  const control = useValidationControl(activeTab, onValidationComplete);
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (event: PointerEvent): void => {
      if (rootRef.current && !rootRef.current.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", onDown, true);
    return () => document.removeEventListener("pointerdown", onDown, true);
  }, [open]);

  return (
    <div className="status-menu" ref={rootRef}>
      <button
        type="button"
        className={`status-menu-button ${open ? "open" : ""}`}
        aria-label="Editor status and actions"
        aria-expanded={open}
        aria-haspopup="menu"
        title="Editor status and actions"
        onClick={() => setOpen((value) => !value)}
      >
        <i className="codicon codicon-ellipsis" aria-hidden="true" />
      </button>
      {open && (
        <div className="status-menu-popover" role="menu" aria-label="Editor status and actions">
          <ValidateItem control={control} />
          <SassWatchItem />
          <StatusMeta tab={activeTab} editorStatus={editorStatus} />
        </div>
      )}
    </div>
  );
}
