import { describe, expect, it, vi } from "vitest";
import { parseCssDiagnostics, parseHtmlDiagnostics, validateWithW3c } from "./w3c-validation";

describe("W3C diagnostic parsing", () => {
  it("maps Nu checker messages to editor positions", () => {
    expect(parseHtmlDiagnostics(JSON.stringify({ messages: [
      { type: "error", firstLine: 3, firstColumn: 4, lastLine: 3, lastColumn: 8, message: "Invalid element" },
      { type: "info", firstLine: 1, firstColumn: 1, message: "Informational" },
      { type: "info", subType: "warning", firstLine: 2, firstColumn: 1, message: "Consider lang" }
    ] }))).toEqual([
      { line: 3, column: 4, endLine: 3, endColumn: 8, message: "Invalid element", severity: "error", source: "w3c-html" },
      { line: 2, column: 1, endLine: 2, endColumn: 2, message: "Consider lang", severity: "warning", source: "w3c-html" }
    ]);
  });

  it("maps legacy CSS validator text output to editor positions", () => {
    expect(parseCssDiagnostics("file://localhost/TextArea:4:.foo:Parse Error\nfile://localhost/TextArea:8:  :A warning"))
      .toEqual([
        { line: 4, column: 1, endLine: 4, endColumn: 2, message: ".foo:Parse Error", severity: "error", source: "w3c-css" },
        { line: 8, column: 1, endLine: 8, endColumn: 2, message: "A warning", severity: "warning", source: "w3c-css" }
      ]);
  });

  it("maps CSS validator SOAP errors and warnings to source lines", () => {
    const response = `<?xml version="1.0"?>
      <env:Envelope xmlns:env="http://www.w3.org/2003/05/soap-envelope">
        <env:Body><m:cssvalidationresponse xmlns:m="http://www.w3.org/2005/07/css-validator">
          <m:result>
            <m:errors><m:errorcount>1</m:errorcount><m:errorlist><m:error>
              <m:line>3</m:line><m:errortype>parse-error</m:errortype><m:message>Property &quot;colr&quot; doesn&apos;t exist &amp; is invalid</m:message>
            </m:error></m:errorlist></m:errors>
            <m:warnings><m:warningcount>1</m:warningcount><m:warninglist><m:warning>
              <m:line>8</m:line><m:level>1</m:level><m:message>Add a generic font family</m:message>
            </m:warning></m:warninglist></m:warnings>
          </m:result>
        </m:cssvalidationresponse></env:Body>
      </env:Envelope>`;

    expect(parseCssDiagnostics(response)).toEqual([
      { line: 3, column: 1, endLine: 3, endColumn: 2, message: 'Property "colr" doesn\'t exist & is invalid', severity: "error", source: "w3c-css" },
      { line: 8, column: 1, endLine: 8, endColumn: 2, message: "Add a generic font family", severity: "warning", source: "w3c-css" }
    ]);
  });

  it("posts CSS source using the validator's SOAP format", async () => {
    const responseBody = `<m:cssvalidationresponse><m:result><m:errors><m:errorcount>0</m:errorcount></m:errors></m:result></m:cssvalidationresponse>`;
    const fetchMock = vi.fn(async (_input: string, _init?: RequestInit) => ({ ok: true, text: async () => responseBody }) as Response);
    vi.stubGlobal("fetch", fetchMock);
    try {
      const source = ".card { color: red; }";
      await expect(validateWithW3c("styles.css", source)).resolves.toEqual([]);
      expect(fetchMock).toHaveBeenCalledWith(
        "https://jigsaw.w3.org/css-validator/validator",
        expect.objectContaining({ method: "POST" })
      );
      const request = fetchMock.mock.calls[0]?.[1] as RequestInit;
      expect(new URLSearchParams(String(request.body)).get("output")).toBe("soap12");
      expect(new URLSearchParams(String(request.body)).get("text")).toBe(source);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("skips preprocessor sources without contacting the validators", async () => {
    await expect(validateWithW3c("src/styles/_welcome.scss", ".a { .b { color: red; } }")).resolves.toEqual([]);
    await expect(validateWithW3c("src/styles/main.less", "@x: 1;")).resolves.toEqual([]);
  });
});
