import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50";

export const secretInputClass = `${inputClass} pr-9 font-mono`;

export const buttonClass =
  "flex items-center gap-1.5 rounded-md border border-border px-3 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-50";

export function Section({
  title,
  icon: Icon,
  children,
}: {
  title: string;
  icon?: LucideIcon;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-border bg-card/50 p-4 sm:p-5">
      <h2 className="mb-3 flex items-center gap-2 border-b border-border pb-3 text-lg font-semibold text-foreground">
        {Icon && <Icon className="h-5 w-5 text-primary" />}
        {title}
      </h2>
      <div className="space-y-4">{children}</div>
    </section>
  );
}

export function Field({ label, children }: { label: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-muted-foreground">{label}</label>
      {children}
    </div>
  );
}

/** Explanatory paragraph shown under a field title. */
export function Explainer({ children }: { children: React.ReactNode }) {
  return (
    <div className="rounded-md border border-border/50 bg-muted/30 px-3 py-2">
      <p className="text-xs leading-relaxed text-muted-foreground">{children}</p>
    </div>
  );
}

export type PillTone = "ok" | "warn" | "error" | "muted";

const PILL_TONES: Record<PillTone, { pill: string; dot: string }> = {
  ok: { pill: "bg-green-500/10 text-green-500", dot: "bg-green-500" },
  warn: { pill: "bg-amber-500/10 text-amber-500", dot: "bg-amber-500" },
  error: { pill: "bg-destructive/10 text-destructive", dot: "bg-destructive" },
  muted: { pill: "bg-muted text-muted-foreground", dot: "bg-muted-foreground" },
};

/** Small status badge with a coloured dot (connection, model, license…). */
export function StatusPill({ tone, children, icon }: { tone: PillTone; children: React.ReactNode; icon?: React.ReactNode }) {
  const c = PILL_TONES[tone];
  return (
    <div className={cn("inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium", c.pill)}>
      {icon ?? <span className={cn("h-1.5 w-1.5 rounded-full", c.dot)} />}
      {children}
    </div>
  );
}

/** Row of mutually exclusive choices rendered as small buttons. */
export function ChoiceRow<T extends string>({
  options,
  value,
  onChange,
}: {
  options: { value: T; label: React.ReactNode; title?: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          title={opt.title}
          onClick={() => onChange(opt.value)}
          className={cn(
            "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors",
            value === opt.value
              ? "border-primary bg-primary/10 text-primary"
              : "border-border text-muted-foreground hover:bg-accent",
          )}
        >
          {opt.label}
        </button>
      ))}
    </div>
  );
}
