import { defineConfig } from "vitest/config";
import path from "path";

// Front-end unit tests: pure TypeScript (stores, lib helpers). No DOM needed —
// components are exercised through the stores they bind to.
export default defineConfig({
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
    clearMocks: true,
  },
});
