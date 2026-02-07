/**
 * Production-grade frontend logger for AIrena.
 *
 * - Structured log entries with timestamp, level, context
 * - Circular buffer (keeps last N entries in memory)
 * - Levels: debug, info, warn, error
 * - Export capability for debugging
 */

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  context: string;
  message: string;
  data?: unknown;
}

const LEVEL_PRIORITY: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

const MAX_ENTRIES = 500;

class Logger {
  private entries: LogEntry[] = [];
  private minLevel: LogLevel = import.meta.env.DEV ? "debug" : "info";

  private push(level: LogLevel, context: string, message: string, data?: unknown) {
    if (LEVEL_PRIORITY[level] < LEVEL_PRIORITY[this.minLevel]) return;

    const entry: LogEntry = {
      timestamp: new Date().toISOString(),
      level,
      context,
      message,
      ...(data !== undefined ? { data } : {}),
    };

    this.entries.push(entry);
    if (this.entries.length > MAX_ENTRIES) {
      this.entries.shift();
    }

    // Also output to console for dev
    const consoleFn = level === "error" ? console.error
      : level === "warn" ? console.warn
      : level === "debug" ? console.debug
      : console.info;
    const prefix = `[${entry.timestamp.slice(11, 23)}] [${level.toUpperCase()}] [${context}]`;
    if (data !== undefined) {
      consoleFn(prefix, message, data);
    } else {
      consoleFn(prefix, message);
    }
  }

  debug(context: string, message: string, data?: unknown) {
    this.push("debug", context, message, data);
  }

  info(context: string, message: string, data?: unknown) {
    this.push("info", context, message, data);
  }

  warn(context: string, message: string, data?: unknown) {
    this.push("warn", context, message, data);
  }

  error(context: string, message: string, data?: unknown) {
    this.push("error", context, message, data);
  }

  /** Get all log entries (most recent last) */
  getEntries(): ReadonlyArray<LogEntry> {
    return this.entries;
  }

  /** Export logs as JSON string (for copy/paste debugging) */
  export(): string {
    return JSON.stringify(this.entries, null, 2);
  }

  /** Clear all entries */
  clear() {
    this.entries = [];
  }
}

/** Singleton logger instance */
export const logger = new Logger();

// Expose globally for console access in production
if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).__airena_logger = logger;
}
