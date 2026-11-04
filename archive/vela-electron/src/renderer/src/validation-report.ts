import type { W3cDiagnostic } from "@shared/types";

export interface ValidationReport {
  path: string;
  diagnostics: W3cDiagnostic[];
  error?: string;
}
