import { useEffect, useState, type ReactNode } from "react";

export function TextShimmer({
  text,
  active = true,
  tone = "default"
}: {
  text: string;
  active?: boolean;
  tone?: "default" | "thinking";
}): ReactNode {
  const [run, setRun] = useState(active);
  useEffect(() => {
    if (active) {
      setRun(true);
      return;
    }
    const timer = setTimeout(() => setRun(false), 220);
    return () => clearTimeout(timer);
  }, [active]);

  return (
    <span data-component="text-shimmer" data-tone={tone} data-active={active ? "true" : "false"} aria-label={text}>
      <span data-slot="text-shimmer-char">
        <span data-slot="text-shimmer-char-base" aria-hidden="true">{text}</span>
        <span data-slot="text-shimmer-char-shimmer" data-run={run ? "true" : "false"} aria-hidden="true">
          {text}
        </span>
      </span>
    </span>
  );
}
