import { defineConfig } from "vitest/config";
import path from "path";

// Front-end unit tests: pure TypeScript (stores, lib helpers) plus the static
// HTML export rendered with react-dom/server. No DOM needed — components are
// exercised through the stores they bind to.
export default defineConfig({
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  test: {
    include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    // Component-free by default; .tsx tests render to static markup (HTML export)
    css: false,
    environment: "node",
    clearMocks: true,
  },
});
