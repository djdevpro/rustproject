import { useEffect, useRef, type ReactNode, type PointerEvent } from "react";

/** Keep the light on the surface, without moving the card or its content. */
export function FeatureCard({ children }: { children: ReactNode }) {
  const frame = useRef<number | undefined>(undefined);
  useEffect(
    () => () => {
      if (frame.current !== undefined) cancelAnimationFrame(frame.current);
    },
    [],
  );
  const light = (event: PointerEvent<HTMLElement>) => {
    if (event.pointerType === "touch") return;
    const element = event.currentTarget;
    const bounds = element.getBoundingClientRect();
    const x = event.clientX - bounds.left;
    const y = event.clientY - bounds.top;
    if (frame.current !== undefined) cancelAnimationFrame(frame.current);
    frame.current = requestAnimationFrame(() => {
      element.style.setProperty("--light-x", `${x}px`);
      element.style.setProperty("--light-y", `${y}px`);
      frame.current = undefined;
    });
  };
  return <article onPointerMove={light}>{children}</article>;
}
