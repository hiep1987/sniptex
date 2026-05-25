export type CapturePoint = { x: number; y: number };
export type CaptureRect = { x: number; y: number; w: number; h: number };

export function CaptureCrosshair({ point }: { point: CapturePoint }) {
  return (
    <>
      <div
        style={{
          position: "absolute",
          left: 0,
          top: point.y,
          width: "100vw",
          height: 1,
          background: "rgba(255, 255, 255, 0.9)",
          boxShadow: "0 0 0 1px rgba(15, 23, 42, 0.45)",
          pointerEvents: "none",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: point.x,
          top: 0,
          width: 1,
          height: "100vh",
          background: "rgba(255, 255, 255, 0.9)",
          boxShadow: "0 0 0 1px rgba(15, 23, 42, 0.45)",
          pointerEvents: "none",
        }}
      />
    </>
  );
}

export function SelectionFrame({ rect }: { rect: CaptureRect }) {
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
        {Math.round(rect.w)} x {Math.round(rect.h)}
      </div>
    </>
  );
}
