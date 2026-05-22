import { useCallback, useEffect, useRef } from "react";

type Options = {
  /** Master switch — when false, no timer arms regardless of other state.
   *  Use to suppress the timer during cold-start / empty preview state. */
  enabled: boolean;
  /** Total ms before `onHide` fires once unpaused. */
  durationMs: number;
  /** Pin sticks the window open until the user dismisses it manually. */
  pinned: boolean;
  /** Pause the timer while the user is hovering anywhere in the window. */
  hovered: boolean;
  /** A monotonically-changing key that resets the timer (new snip = new key). */
  resetKey: unknown;
  /** Fires when the timer expires unpaused. */
  onHide: () => void;
};

/**
 * Drive a single auto-hide timer for a floating window.
 *
 * Behaviour:
 *   - `pinned` disables the timer entirely.
 *   - `hovered` pauses; un-hovering restarts the full duration.
 *   - `resetKey` changing restarts the timer from zero.
 *   - The caller can also force-extend the timer via the returned
 *     `bump()` (called from Copy so the user sees the toast complete
 *     before the window disappears).
 */
export function useAutoHide({
  enabled,
  durationMs,
  pinned,
  hovered,
  resetKey,
  onHide,
}: Options): { bump: () => void } {
  const timerRef = useRef<number | null>(null);
  // Mirror the onHide callback so timer-effect doesn't re-arm whenever
  // the parent re-renders with a fresh closure.
  const hideRef = useRef(onHide);
  hideRef.current = onHide;

  const clearTimer = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const armTimer = useCallback(() => {
    clearTimer();
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      hideRef.current();
    }, durationMs);
  }, [clearTimer, durationMs]);

  useEffect(() => {
    if (!enabled || pinned || hovered) {
      clearTimer();
      return;
    }
    armTimer();
    return clearTimer;
  }, [enabled, pinned, hovered, resetKey, armTimer, clearTimer]);

  const bump = useCallback(() => {
    if (!enabled || pinned || hovered) return;
    armTimer();
  }, [enabled, pinned, hovered, armTimer]);

  return { bump };
}
