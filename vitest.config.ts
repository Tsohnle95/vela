import { defineConfig } from "vitest/config";
import { resolve } from "node:path";

export default defineConfig({
  resolve: {
    alias: [
      { find: /^monaco-editor$/, replacement: resolve(__dirname, "node_modules/monaco-editor/esm/vs/editor/editor.main.js") },
      { find: "@shared", replacement: resolve(__dirname, "src/shared") },
      { find: "@", replacement: resolve(__dirname, "src/renderer/src") }
    ]
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
    exclude: [
      "**/node_modules/**",
      "**/dist/**",
      "**/out/**",
      "**/release/**",
      "**/test-video/**",
      "**/cypress/**",
      "**/.{idea,git,cache,output,temp}/**"
    ]
  }
});
