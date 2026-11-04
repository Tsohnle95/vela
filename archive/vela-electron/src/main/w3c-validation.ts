import type { W3cDiagnostic } from "@shared/types";

const HTML_VALIDATOR_URL = "https://validator.w3.org/nu/?out=json";
const CSS_VALIDATOR_URL = "https://jigsaw.w3.org/css-validator/validator";
const MAX_HTML_BYTES = 4 * 1024 * 1024;
const MAX_CSS_BYTES = 200 * 1024;

type NuMessage = {
  type?: unknown;
  subType?: unknown;
  message?: unknown;
  firstLine?: unknown;
  firstColumn?: unknown;
  lastLine?: unknown;
  lastColumn?: unknown;
};

function numberValue(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value > 0 ? value : fallback;
}

export function parseHtmlDiagnostics(body: string): W3cDiagnostic[] {
  const parsed = JSON.parse(body) as { messages?: unknown };
  if (!Array.isArray(parsed.messages)) return [];
  return parsed.messages.flatMap((value): W3cDiagnostic[] => {
    if (!value || typeof value !== "object") return [];
    const message = value as NuMessage;
    if (typeof message.message !== "string" || (message.type === "info" && message.subType !== "warning")) return [];
    return [{
      line: numberValue(message.firstLine, 1),
      column: numberValue(message.firstColumn, 1),
      endLine: numberValue(message.lastLine, numberValue(message.firstLine, 1)),
      endColumn: numberValue(message.lastColumn, numberValue(message.firstColumn, 1) + 1),
      message: message.message,
      severity: message.subType === "warning" ? "warning" : "error",
      source: "w3c-html"
    }];
  });
}

export function parseCssDiagnostics(body: string): W3cDiagnostic[] {
  if (/<(?:[\w.-]+:)?cssvalidationresponse\b/i.test(body)) {
    const diagnostics: W3cDiagnostic[] = [];
    const decodeXml = (value: string): string => value
      .replace(/&lt;/g, "<")
      .replace(/&gt;/g, ">")
      .replace(/&quot;/g, '"')
      .replace(/&apos;/g, "'")
      .replace(/&#(\d+);/g, (_match, decimal: string) => String.fromCodePoint(Number(decimal)))
      .replace(/&#x([\da-f]+);/gi, (_match, hex: string) => String.fromCodePoint(parseInt(hex, 16)))
      .replace(/&amp;/g, "&");
    const tagText = (xml: string, name: string): string => {
      const expression = new RegExp(`<(?:(?:[\\w.-]+):)?${name}\\b[^>]*>([\\s\\S]*?)<\\/(?:(?:[\\w.-]+):)?${name}\\s*>`, "i");
      const match = expression.exec(xml);
      return match ? decodeXml(match[1].replace(/<[^>]*>/g, "").trim()) : "";
    };
    const parseItems = (tag: "error" | "warning", severity: "error" | "warning"): void => {
      const expression = new RegExp(`<(?:(?:[\\w.-]+):)?${tag}\\b[^>]*>([\\s\\S]*?)<\\/(?:(?:[\\w.-]+):)?${tag}\\s*>`, "gi");
      for (const match of body.matchAll(expression)) {
        const block = match[1];
        const line = numberValue(Number(tagText(block, "line")), 1);
        const message = tagText(block, "message") || tagText(block, "errortype") || `CSS validation ${severity}`;
        diagnostics.push({
          line,
          column: 1,
          endLine: line,
          endColumn: 2,
          message,
          severity,
          source: "w3c-css"
        });
      }
    };
    parseItems("error", "error");
    parseItems("warning", "warning");
    if (diagnostics.length === 0 && /<(?:(?:[\w.-]+):)?validity\b[^>]*>\s*false\s*</i.test(body)) {
      return [{
        line: 1,
        column: 1,
        endLine: 1,
        endColumn: 2,
        message: "The CSS validator reported errors but did not return their details.",
        severity: "error",
        source: "w3c-css"
      }];
    }
    return diagnostics;
  }

  return body.split(/\r?\n/).flatMap((line): W3cDiagnostic[] => {
    const match = /^.*?:(\d+)(?::(\d+))?:\s*(.*)$/.exec(line);
    if (!match || !match[3]) return [];
    const message = match[3].startsWith(":") ? match[3].slice(1) : match[3];
    return [{
      line: Number(match[1]),
      column: match[2] ? Number(match[2]) : 1,
      endLine: Number(match[1]),
      endColumn: (match[2] ? Number(match[2]) : 1) + 1,
      message,
      severity: /warning/i.test(message) ? "warning" : "error",
      source: "w3c-css"
    }];
  });
}

async function request(url: string, init?: RequestInit): Promise<Response> {
  return fetch(url, { ...init, signal: AbortSignal.timeout(15_000), headers: { Accept: "application/json, text/plain", ...init?.headers } });
}

export async function validateWithW3c(path: string, content: string): Promise<W3cDiagnostic[]> {
  const lower = path.toLowerCase();
  if (lower.endsWith(".html") || lower.endsWith(".htm")) {
    if (Buffer.byteLength(content, "utf8") > MAX_HTML_BYTES) throw new Error("HTML validation is limited to 4 MiB.");
    const response = await request(HTML_VALIDATOR_URL, {
      method: "POST",
      headers: { "Content-Type": "text/html; charset=utf-8" },
      body: content
    });
    if (!response.ok) throw new Error(`W3C HTML validator returned ${response.status}`);
    return parseHtmlDiagnostics(await response.text());
  }
  if (!lower.endsWith(".css")) return [];
  if (Buffer.byteLength(content, "utf8") > MAX_CSS_BYTES) throw new Error("CSS validation is limited to 200 KiB.");
  const params = new URLSearchParams({ output: "soap12", profile: "css3", warning: "2", lang: "en", text: content });
  const response = await request(CSS_VALIDATOR_URL, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded; charset=utf-8", Accept: "application/soap+xml, text/plain" },
    body: params.toString()
  });
  if (!response.ok) throw new Error(`W3C CSS validator returned ${response.status}`);
  return parseCssDiagnostics(await response.text());
}
