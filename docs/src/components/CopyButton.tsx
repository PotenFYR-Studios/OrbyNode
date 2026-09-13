// Shared copy-to-clipboard button (hero install command). Shows
// "Copied!" for ~1.4s. Dependency-free.
import { useRef, useState } from "react";

export function CopyButton({ text }: { text: string }) {
  const [ok, setOk] = useState(false);
  const timer = useRef<number>(0);

  return (
    <button
      type="button"
      className={`copy-btn${ok ? " ok" : ""}`}
      onClick={() => {
        void navigator.clipboard?.writeText(text).then(() => {
          setOk(true);
          window.clearTimeout(timer.current);
          timer.current = window.setTimeout(() => setOk(false), 1400);
        });
      }}
    >
      {ok ? "Copied!" : "Copy"}
    </button>
  );
}
