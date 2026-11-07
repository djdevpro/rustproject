import { useEffect, useRef, type ReactNode } from "react";

export function MacComputer({ children }: { children: ReactNode }) {
  const screen = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = screen.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      element.style.setProperty(
        "--screen-scale",
        String(entry.contentRect.width / 1480),
      );
    });
    observer.observe(element);
    return () => observer.disconnect();
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
