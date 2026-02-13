/**
 * Extract a human-readable error message from any caught value.
 *
 * Tauri commands serialize `CommandError` as `{ kind: "Rag", message: "..." }`.
 * Plain `String(e)` on such objects produces "[object Object]".
 * This utility handles all known shapes:
 *   - Error instances → e.message
 *   - Tauri CommandError objects → e.message (the string field)
 *   - Plain strings → as-is
 *   - Anything else → JSON.stringify fallback
 */
export function extractErrorMessage(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  if (typeof e === "object" && e !== null) {
    // Tauri CommandError: { kind: string, message: string }
    const obj = e as Record<string, unknown>;
    if (typeof obj.message === "string") return obj.message;
    // Other object shapes — try JSON
    try {
      return JSON.stringify(e);
    } catch {
      return "[unknown error]";
    }
  }
  return String(e);
}
