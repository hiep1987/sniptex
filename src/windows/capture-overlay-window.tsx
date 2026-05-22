import { useEffect, useRef, useState } from "react";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";

type CaptureStart = {
  backdrop_path: string;
  logical_width: number;
  logical_height: number;
  pixel_width: number;
  pixel_height: number;
  scale_factor: number;
};

type Rect = { x: number; y: number; w: number; h: number };

const CAPTURE_START = "capture-start";
const CAPTURE_REGION = "capture-region";
const CAPTURE_CANCEL = "capture-cancel";

export default function CaptureOverlayWindow() {
  const [backdrop, setBackdrop] = useState<string | null>(null);
  const [size, setSize] = useState<{ w: number; h: number }>({ w: 0, h: 0 });
  const [drag, setDrag] = useState<Rect | null>(null);
  const dragStart = useRef<{ x: number; y: number } | null>(null);
  // Mirror `drag` into a ref so the keydown handler can read the latest
  // selection without re-binding on every mouse-move tick.
  const dragRef = useRef<Rect | null>(null);
  dragRef.current = drag;

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    listen<CaptureStart>(CAPTURE_START, (event) => {
      const p = event.payload;
      setBackdrop(convertFileSrc(p.backdrop_path));
      setSize({ w: p.logical_width, h: p.logical_height });
      setDrag(null);
      dragStart.current = null;
    })
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch((e) => console.error("[overlay] listen capture-start failed", e));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        void emit(CAPTURE_CANCEL);
        resetDrag();
      } else if (e.key === "Enter") {
        const current = dragRef.current;
        if (current) {
          e.preventDefault();
          commit(current);
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  function resetDrag() {
    setDrag(null);
    dragStart.current = null;
  }

  function commit(rect: Rect) {
    if (rect.w <= 1 || rect.h <= 1) {
      resetDrag();
      return;
    }
    void emit(CAPTURE_REGION, rect);
    resetDrag();
  }

  function onMouseDown(e: React.MouseEvent) {
    if (e.button !== 0) return;
    dragStart.current = { x: e.clientX, y: e.clientY };
    setDrag({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
  }

  function onMouseMove(e: React.MouseEvent) {
    if (!dragStart.current) return;
    const { x: sx, y: sy } = dragStart.current;
    const x = Math.min(sx, e.clientX);
    const y = Math.min(sy, e.clientY);
    const w = Math.abs(e.clientX - sx);
    const h = Math.abs(e.clientY - sy);
    setDrag({ x, y, w, h });
  }

  function onMouseUp() {
    if (!drag || !dragStart.current) return;
    commit(drag);
  }

  if (!backdrop) {
    return <div style={fullscreenContainerStyle} />;
  }

  return (
    <div
      style={fullscreenContainerStyle}
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={onMouseUp}
    >
      <img
        src={backdrop}
        alt=""
        draggable={false}
        style={{
          position: "absolute",
          inset: 0,
          width: size.w || "100vw",
          height: size.h || "100vh",
          userSelect: "none",
          pointerEvents: "none",
        }}
      />
      {/* Dim mask over the whole screen; the selection cuts a hole via box-shadow. */}
      <div style={dimMaskStyle(drag)} />
      {drag && drag.w > 0 && drag.h > 0 && (
        <SelectionFrame rect={drag} />
      )}
    </div>
  );
}

function SelectionFrame({ rect }: { rect: Rect }) {
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: rect.x,
          top: rect.y,
          width: rect.w,
          height: rect.h,
          border: "1.5px solid rgb(59, 130, 246)",
          boxSizing: "border-box",
          pointerEvents: "none",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: rect.x + 4,
          top: Math.max(0, rect.y - 22),
          padding: "2px 6px",
          fontSize: 11,
          fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
          color: "white",
          background: "rgba(59, 130, 246, 0.9)",
          borderRadius: 3,
          pointerEvents: "none",
        }}
      >
        {Math.round(rect.w)} × {Math.round(rect.h)}
      </div>
    </>
  );
}

const fullscreenContainerStyle: React.CSSProperties = {
  position: "fixed",
  inset: 0,
  width: "100vw",
  height: "100vh",
  margin: 0,
  padding: 0,
  cursor: "crosshair",
  overflow: "hidden",
  background: "transparent",
};

function dimMaskStyle(rect: Rect | null): React.CSSProperties {
  // No selection yet → dim the whole screen uniformly.
  if (!rect || rect.w === 0 || rect.h === 0) {
    return {
      position: "absolute",
      inset: 0,
      background: "rgba(0, 0, 0, 0.35)",
      pointerEvents: "none",
    };
  }
  // With a selection, use a giant inset box-shadow to dim everything outside
  // the rect — single element, no four-strip math, GPU-cheap.
  return {
    position: "absolute",
    left: rect.x,
    top: rect.y,
    width: rect.w,
    height: rect.h,
    boxShadow: "0 0 0 100000px rgba(0, 0, 0, 0.35)",
    pointerEvents: "none",
  };
}
