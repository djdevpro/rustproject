import { useEffect, useRef, type ReactNode } from "react";

export function MacComputer({ children }: { children: ReactNode }) {
  const screen = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = screen.current;
    if (!element) return;
    let width = 0;
    let frame = 0;
    const observer = new ResizeObserver(([entry]) => {
      if (entry.contentRect.width === width) return;
      width = entry.contentRect.width;
      // Scaling also changes the screen height. Write outside observer delivery
      // so WebKit does not report a ResizeObserver feedback loop.
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        element.style.setProperty("--screen-scale", String(width / 1480));
      });
    });
    observer.observe(element);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, []);
  return (
    <div className="product-preview">
      <div className="mac-computer">
        <div className="mac-screen" ref={screen}>
          {children}
        </div>
        <div className="mac-base" aria-hidden="true" />
      </div>
    </div>
  );
}
