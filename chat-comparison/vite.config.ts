import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";
export default defineConfig({
  base: "./",
  root: resolve(__dirname, ".."),
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": resolve(__dirname, "../src/renderer/src"), "@shared": resolve(__dirname, "../src/shared") } },
  worker: { format: "es" },
  server: { host: "127.0.0.1", port: 5280, strictPort: true },
  build: { outDir: "out/chat-comparison", rollupOptions: {input: {index:resolve(__dirname,"index.html"),frame:resolve(__dirname,"frame.html")}} }
});
