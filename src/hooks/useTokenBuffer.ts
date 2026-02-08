import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Buffer streaming tokens and flush them every `intervalMs` to avoid
 * re-rendering React on every single token (which can crash the WebView).
 *
 * Instead of 200+ state updates per speaker, this produces ~1 update per
 * 60ms interval — a 100x reduction in render frequency.
 */
export function useTokenBuffer(intervalMs = 60) {
  const bufferRef = useRef<Map<string, string[]>>(new Map());
  const [flushed, setFlushed] = useState<Map<string, string>>(new Map());

  const pushToken = useCallback((speakerId: string, token: string) => {
    const buffer = bufferRef.current;
    if (!buffer.has(speakerId)) buffer.set(speakerId, []);
    buffer.get(speakerId)!.push(token);
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      const buffer = bufferRef.current;
      if (buffer.size === 0) return;

      setFlushed((prev) => {
        const next = new Map(prev);
        buffer.forEach((tokens, speakerId) => {
          next.set(speakerId, (next.get(speakerId) ?? "") + tokens.join(""));
        });
        buffer.clear();
        return next;
      });
    }, intervalMs);
    return () => clearInterval(interval);
  }, [intervalMs]);

  const clearSpeaker = useCallback((speakerId: string) => {
    bufferRef.current.delete(speakerId);
    setFlushed((prev) => {
      const next = new Map(prev);
      next.delete(speakerId);
      return next;
    });
  }, []);

  const clearAll = useCallback(() => {
    bufferRef.current.clear();
    setFlushed(new Map());
  }, []);

  return { flushed, pushToken, clearSpeaker, clearAll };
}
