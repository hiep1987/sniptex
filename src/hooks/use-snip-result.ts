import { useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { SnipResult } from "@/lib/invoke";

export type SnipEvent = {
  /** Monotonic counter that increments on every backend emission. Use
   *  this as a resetKey for timers so identical-content snips still
   *  count as fresh events. */
  seq: number;
  result: SnipResult;
};

/**
 * Subscribe to the backend `snip-complete` event and surface the latest
 * payload along with a monotonic sequence number. The sequence guards
 * downstream `useEffect`/timer reset logic against two consecutive
 * snips that happen to produce identical OCR text or image paths.
 */
export function useSnipResult(): SnipEvent | null {
  const [event, setEvent] = useState<SnipEvent | null>(null);

  useEffect(() => {
    let cancelled = false;
    let offSnip: UnlistenFn | undefined;
    let offHistory: UnlistenFn | undefined;
    let seq = 0;

    const handleResult = (result: SnipResult) => {
      // Guarded against listener-resolution-after-unmount under React
      // StrictMode's double-invoke effect cycle in dev.
      if (cancelled) return;
      seq += 1;
      setEvent({ seq, result });
    };

    listen<SnipResult>("snip-complete", (e) => {
      handleResult(e.payload);
    })
      .then((fn) => {
        if (cancelled) fn();
        else offSnip = fn;
      })
      .catch((err) =>
        console.error("[snip-result] snip-complete listen failed", err),
      );

    listen<SnipResult>("history-preview-open", (e) => {
      handleResult(e.payload);
    })
      .then((fn) => {
        if (cancelled) fn();
        else offHistory = fn;
      })
      .catch((err) =>
        console.error("[snip-result] history-preview-open listen failed", err),
      );

    return () => {
      cancelled = true;
      offSnip?.();
      offHistory?.();
    };
  }, []);

  return event;
}
