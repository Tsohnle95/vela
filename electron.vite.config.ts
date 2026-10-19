import { defineConfig, externalizeDepsPlugin } from "electron-vite";
import react from "@vitejs/plugin-react";
import { resolve } from "node:path";
import { editableCssSourceMaps } from "./scripts/editable-css-source-maps";

const isReleaseBuild = (process.env.VELA_RELEASE_BUILD ?? process.env.VELA_RELEASE_BUILD) === "1";

export default defineConfig({
  main: {
    build: {
      rollupOptions: { input: { index: resolve(__dirname, "src/main/index.ts"), "sass-worker": resolve(__dirname, "src/main/sass-worker.ts") } }
    },
    define: {
      __VELA_RELEASE_BUILD__: JSON.stringify(isReleaseBuild)
    },
    resolve: {
      alias: {
        "@shared": resolve(__dirname, "src/shared")
      }
    },
    plugins: [externalizeDepsPlugin({ exclude: ["@opencode/client"] })]
  },
  preload: {
    plugins: [externalizeDepsPlugin()]
  },
  renderer: {
    build: { cssMinify: false },
    css: {
      devSourcemap: true
    },
    worker: {
      format: "es"
    },
    resolve: {
      alias: {
        "@": resolve(__dirname, "src/renderer/src"),
        "@shared": resolve(__dirname, "src/shared")
      }
    },
    plugins: [react(), editableCssSourceMaps(resolve(__dirname, "src/renderer/src/styles/main.scss"))]
  }
});
