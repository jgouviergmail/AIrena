import { useCallback, useRef } from "react";

export function ResizeDivider({ onResize }: { onResize: (delta: number) => void }) {
  const dragging = useRef(false);
  const lastX = useRef(0);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      dragging.current = true;
      lastX.current = e.clientX;

      const handleMouseMove = (ev: MouseEvent) => {
        if (!dragging.current) return;
        const delta = lastX.current - ev.clientX;
        lastX.current = ev.clientX;
        onResize(delta);
      };

      const handleMouseUp = () => {
        dragging.current = false;
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
        document.body.style.cursor = "";
        document.body.style.userSelect = "";
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [onResize],
  );

  return (
    <div
      onMouseDown={handleMouseDown}
      className="group flex h-full w-1 shrink-0 cursor-col-resize items-center justify-center transition-colors hover:bg-primary/20"
    >
      <div className="h-8 w-0.5 rounded-full bg-border transition-colors group-hover:bg-primary" />
    </div>
  );
}
