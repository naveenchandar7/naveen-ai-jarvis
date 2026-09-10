import { useEffect, useRef } from "react";
import { useTheme } from "../../hooks/useTheme";

export function VirtualCursor() {
  const dotRef = useRef<HTMLDivElement>(null);
  const theme = useTheme();

  useEffect(() => {
    function handleMove(event: MouseEvent) {
      const dot = dotRef.current;
      if (!dot) return;

      dot.style.transform = `translate3d(${event.clientX}px, ${event.clientY}px, 0)`;
    }

    window.addEventListener("pointermove", handleMove);
    return () => window.removeEventListener("pointermove", handleMove);
  }, []);

  return (
    <div
      ref={dotRef}
      className="virtual-cursor"
      style={{ boxShadow: `0 0 12px ${theme.primary}, 0 0 24px ${theme.shadow}` }}
    />
  );
}
