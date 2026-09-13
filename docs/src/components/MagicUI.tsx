// Locally owned Magic UI-style effects, dependency-free (pure CSS/rAF).
// Landing hero only - no effects inside doc articles. All animations are
// disabled by prefers-reduced-motion via app.css.
import { useEffect, useState } from "react";

/** Magic UI - Number Ticker: counts up to `value` with rAF easing.
 * Starts at `value` so SSR/prerendered markup matches (no hydration
 * mismatch), then restarts the count on mount in the browser. */
export function NumberTicker({
  value,
  className,
}: {
  value: number;
  className?: string;
}) {
  const [display, setDisplay] = useState(value);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    const start = performance.now();
    const duration = 900;
    let raf = 0;
    const tick = (now: number) => {
      const t = Math.min((now - start) / duration, 1);
      const eased = 1 - Math.pow(1 - t, 3);
      setDisplay(Math.round(value * eased));
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [value]);

  return (
    <span className={className} aria-hidden={mounted ? undefined : "false"}>
      {display}
    </span>
  );
}

/** Ambient glow orb for hero atmosphere. */
export function GlowOrb({
  className = "",
  color = "rgba(34, 211, 238, 0.16)",
  size = 420,
}: {
  className?: string;
  color?: string;
  size?: number;
}) {
  return (
    <div
      aria-hidden="true"
      className={`orb ${className}`}
      style={{ width: size, height: size, background: color }}
    />
  );
}

/** Spotlight card: pointer-following radial highlight (CSS-driven). */
export function SpotlightCard({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`magic-card ${className}`}
      onPointerMove={(e) => {
        const rect = e.currentTarget.getBoundingClientRect();
        e.currentTarget.style.setProperty("--mx", `${e.clientX - rect.left}px`);
        e.currentTarget.style.setProperty("--my", `${e.clientY - rect.top}px`);
      }}
    >
      {children}
    </div>
  );
}
