// Token streams live outside Zustand: a per-token `set()` crashes the WebView.
// Message/reasoning tokens go to the DiscussionFeed buffer (60 ms flush);
// synthesis tokens to a module-level buffer flushed on the same cadence.

/** Buffer key of a speaker's live reasoning stream (separate from their content stream). */
const REASONING_STREAM_SUFFIX = "::reasoning";
export const reasoningStreamKey = (speakerId: string) => `${speakerId}${REASONING_STREAM_SUFFIX}`;

/** Split a buffer key back into speaker id + stream kind. */
export function parseStreamKey(key: string): { speakerId: string; kind: "content" | "reasoning" } {
  return key.endsWith(REASONING_STREAM_SUFFIX)
    ? { speakerId: key.slice(0, -REASONING_STREAM_SUFFIX.length), kind: "reasoning" }
    : { speakerId: key, kind: "content" };
}

/** Callbacks from the token buffer in DiscussionFeed */
export interface StreamBufferCallbacks {
  pushToken: (speakerId: string, token: string) => void;
  clearSpeaker: (speakerId: string) => void;
  clearAll: () => void;
}

const SYNTH_FLUSH_MS = 60;

/** Synthesis token buffer: collects chunks and flushes them through `apply` every 60 ms. */
export class SynthesisBuffer {
  private chunks: string[] = [];
  private timer: ReturnType<typeof setInterval> | null = null;

  constructor(private readonly apply: (chunk: string) => void) {}

  /** True when a flush timer is running (i.e. a synthesis is streaming). */
  get active(): boolean {
    return this.timer !== null;
  }

  push(chunk: string) {
    this.chunks.push(chunk);
    if (!this.timer) this.timer = setInterval(() => this.flush(), SYNTH_FLUSH_MS);
  }

  private flush() {
    if (this.chunks.length === 0) return;
    const chunk = this.chunks.join("");
    this.chunks = [];
    this.apply(chunk);
  }

  /** Stop the timer and deliver whatever is left. */
  stop() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
    this.flush();
    this.chunks = [];
  }
}
