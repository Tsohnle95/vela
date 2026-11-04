import { Fragment, useEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";

// Ports of OpenCode's `animated-number`, `tool-count-label`, and
// `tool-count-summary` components. English-only: Vela has no i18n layer, so the
// plural copy is inlined from OpenCode's `en.ts`.

const TRACK = Array.from({ length: 30 }, (_, index) => index % 10);
const ODOMETER_DURATION = 600;

function normalize(value: number): number {
  return ((value % 10) + 10) % 10;
}

function spin(from: number, to: number, direction: 1 | -1): number {
  if (from === to) return 0;
  if (direction > 0) return (to - from + 10) % 10;
  return -((from - to + 10) % 10);
}

function Digit({ value, direction }: { value: number; direction: 1 | -1 }): ReactNode {
  const [step, setStep] = useState(value + 10);
  const [animating, setAnimating] = useState(false);
  const lastRef = useRef(value);
  const mountedRef = useRef(false);

  useEffect(() => {
    if (!mountedRef.current) {
      mountedRef.current = true;
      lastRef.current = value;
      return;
    }
    const delta = spin(lastRef.current, value, direction);
    lastRef.current = value;
    if (!delta) {
      setAnimating(false);
      setStep(value + 10);
      return;
    }
    setAnimating(true);
    setStep((current) => current + delta);
  }, [value, direction]);

  const style = {
    "--animated-number-offset": `${step}`,
    "--animated-number-duration": `var(--tool-motion-odometer-ms, ${ODOMETER_DURATION}ms)`
  } as CSSProperties;

  return (
    <span data-slot="animated-number-digit">
      <span
        data-slot="animated-number-strip"
        data-animating={animating ? "true" : "false"}
        onTransitionEnd={() => {
          setAnimating(false);
          setStep((current) => normalize(current) + 10);
        }}
        style={style}
      >
        {TRACK.map((cell, index) => <span data-slot="animated-number-cell" key={index}>{cell}</span>)}
      </span>
    </span>
  );
}

export function AnimatedNumber({ value, className }: { value: number; className?: string }): ReactNode {
  const target = Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
  const [state, setState] = useState({ value: target, direction: 1 as 1 | -1 });
  const valueRef = useRef(target);
  const mountedRef = useRef(false);

  useEffect(() => {
    if (!mountedRef.current) {
      mountedRef.current = true;
      valueRef.current = target;
      return;
    }
    const current = valueRef.current;
    if (target === current) return;
    valueRef.current = target;
    setState({ direction: target > current ? 1 : -1, value: target });
  }, [target]);

  const label = state.value.toString();
  const digits = Array.from(label, (char) => {
    const code = char.charCodeAt(0) - 48;
    return code < 0 || code > 9 ? 0 : code;
  }).reverse();

  return (
    <span data-component="animated-number" className={className} aria-label={label}>
      <span data-slot="animated-number-value" style={{ "--animated-number-width": `${digits.length}ch` } as CSSProperties}>
        {digits.map((digit, index) => (
          <Digit value={digit} direction={state.direction} key={digits.length - index} />
        ))}
      </span>
    </span>
  );
}

function split(text: string): { before: string; after: string } {
  const match = /{{\s*count\s*}}/.exec(text);
  if (!match || match.index === undefined) return { before: "", after: text };
  return { before: text.slice(0, match.index), after: text.slice(match.index + match[0].length) };
}

function common(one: string, other: string): { stem: string; one: string; other: string } {
  const a = Array.from(one);
  const b = Array.from(other);
  let i = 0;
  while (i < a.length && i < b.length && a[i] === b[i]) i += 1;
  return { stem: a.slice(0, i).join(""), one: a.slice(i).join(""), other: b.slice(i).join("") };
}

export type CountPlural = "read" | "search" | "list";

const PLURALS: Record<CountPlural, { one: string; other: string }> = {
  read: { one: "{{count}} read", other: "{{count}} reads" },
  search: { one: "{{count}} search", other: "{{count}} searches" },
  list: { one: "{{count}} list", other: "{{count}} lists" }
};

export function AnimatedCountLabel({ count, plural }: { count: number; plural: CountPlural }): ReactNode {
  const one = split(PLURALS[plural].one);
  const other = split(PLURALS[plural].other);
  const rounded = Math.round(count);
  const category = rounded === 1 ? "one" : "other";
  const active = category === "one" ? one : other;
  const suffix = common(one.after, other.after);
  const splitSuffix =
    one.before === other.before &&
    (one.after.startsWith(other.after) || other.after.startsWith(one.after));
  const before = splitSuffix ? one.before : active.before;
  const stem = splitSuffix ? suffix.stem : active.after;
  const tail = !splitSuffix ? "" : category === "one" ? suffix.one : suffix.other;
  const showTail = splitSuffix && tail.length > 0;

  return (
    <span data-component="tool-count-label">
      <span data-slot="tool-count-label-before">{before}</span>
      <AnimatedNumber value={rounded} />
      <span data-slot="tool-count-label-word">
        <span data-slot="tool-count-label-stem">{stem}</span>
        <span data-slot="tool-count-label-suffix" data-active={showTail ? "true" : "false"}>
          <span data-slot="tool-count-label-suffix-inner">{tail}</span>
        </span>
      </span>
    </span>
  );
}

export type CountItem = { key: CountPlural; count: number };

export function AnimatedCountList({
  items,
  fallback = "",
  className
}: {
  items: CountItem[];
  fallback?: string;
  className?: string;
}): ReactNode {
  const showEmpty = items.every((item) => item.count <= 0) && fallback.length > 0;

  return (
    <span data-component="tool-count-summary" className={className}>
      <span data-slot="tool-count-summary-empty" data-active={showEmpty ? "true" : "false"}>
        <span data-slot="tool-count-summary-empty-inner">{fallback}</span>
      </span>
      {items.map((item, index) => {
        const active = item.count > 0;
        let hasPrev = false;
        for (let i = index - 1; i >= 0; i -= 1) {
          if (items[i].count > 0) {
            hasPrev = true;
            break;
          }
        }
        return (
          <Fragment key={item.key}>
            <span data-slot="tool-count-summary-prefix" data-active={active && hasPrev ? "true" : "false"}>,</span>
            <span data-slot="tool-count-summary-item" data-active={active ? "true" : "false"}>
              <span data-slot="tool-count-summary-item-inner">
                <AnimatedCountLabel plural={item.key} count={Math.max(0, Math.round(item.count))} />
              </span>
            </span>
          </Fragment>
        );
      })}
    </span>
  );
}
