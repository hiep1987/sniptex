import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import { toast } from "sonner";
import { tauri } from "@/lib/invoke";
import { useHotkeyStore } from "@/state/hotkey-store";

/**
 * Wire the global hotkey + tray-snip-now events to the `run_snip`
 * command. Owns its own re-entrancy guard so a fast double-tap of the
 * hotkey doesn't race two captures against the single shared overlay
 * (the Rust side has its own guard too — defense in depth).
 *
 * Mount once per window. Preview Window mounts this on its root so
 * the snip pipeline runs even while the preview is hidden.
 */
export function useSnipTrigger() {
  const recordPress = useHotkeyStore((s) => s.recordPress);
  const inFlight = useRef(false);

  useEffect(() => {
    let cancelled = false;
    const offs: UnlistenFn[] = [];

    const triggerSnip = async () => {
      if (inFlight.current) return;
      inFlight.current = true;
      try {
        const result = await tauri.runSnip();
        if (result.status === "cancelled") {
          toast("Snip cancelled");
        }
        // On success, Rust emits `snip-complete` which Preview Window's
        // useSnipResult picks up — no further action here.
      } catch (err) {
        const msg = String(err);
        // Toasts render inside whichever window hosts <Toaster /> — when
        // that window is hidden (Preview at startup), the user sees
        // nothing. Mirror the error to an OS notification so it surfaces
        // regardless of window visibility.
        toast.error("Snip failed", { description: msg });
        void notifySnipFailure(msg);
      } finally {
        inFlight.current = false;
      }
    };

    // Subscribe each listener independently so a single failed
    // registration doesn't leak the others — Promise.all on
    // listen() would lose the successful handles on a partial reject.
    const register = <T>(name: string, handler: (e: { payload: T }) => void) => {
      listen<T>(name, handler)
        .then((fn) => {
          if (cancelled) fn();
          else offs.push(fn);
        })
        .catch((err) =>
          console.error(`[snip-trigger] listen(${name}) failed`, err),
        );
    };

    register<unknown>("hotkey-pressed", () => {
      recordPress();
      void triggerSnip();
    });
    register<unknown>("tray-snip-now", () => {
      void triggerSnip();
    });
    register<{ shortcut: string; reason: string }>("hotkey-conflict", (e) => {
      toast.error("Hotkey unavailable", {
        description: e.payload.reason,
        duration: 8000,
      });
    });

    return () => {
      cancelled = true;
      for (const off of offs) off();
    };
  }, [recordPress]);
}

async function notifySnipFailure(message: string): Promise<void> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const res = await requestPermission();
      granted = res === "granted";
    }
    if (granted) {
      sendNotification({ title: "SnipTeX — snip failed", body: message });
    }
  } catch (err) {
    console.warn("[snip-trigger] notification failed", err);
  }
}
