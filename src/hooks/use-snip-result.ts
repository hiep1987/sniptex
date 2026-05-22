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
    let off: UnlistenFn | undefined;
    let seq = 0;

    listen<SnipResult>("snip-complete", (e) => {
      // Guarded against listener-resolution-after-unmount under React
      // StrictMode's double-invoke effect cycle in dev.
      if (cancelled) return;
      seq += 1;
      setEvent({ seq, result: e.payload });
    })
      .then((fn) => {
        if (cancelled) fn();
        else off = fn;
      })
      .catch((err) => console.error("[snip-result] listen failed", err));

    return () => {
      cancelled = true;
      off?.();
    };
  }, []);

  return event;
}
