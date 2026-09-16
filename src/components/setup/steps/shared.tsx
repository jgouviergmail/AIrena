import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import type { DiscussionMode, DocumentFormat } from "@/lib/types";

export const DISCUSSION_MODES: DiscussionMode[] = [
  "debate", "ideation", "coConstruction", "userDriven",
  "socratic", "tutorial", "critiqueReview", "collaborativeFiction",
];

export const DOCUMENT_FORMATS: DocumentFormat[] = ["none", "txt", "md", "csv"];

export const RAG_SUPPORTED_EXTENSIONS = new Set([
  "txt", "pdf", "docx", "pptx",
  "py", "rs", "ts", "tsx", "js", "jsx", "java", "c", "cpp", "go", "rb", "php",
  "swift", "kt", "cs", "yaml", "yml", "json", "xml", "html", "css", "sql",
  "md", "csv", "toml", "sh", "log",
]);

/** Underlined section title with its icon (shared by every wizard step). */
export function SectionLabel({
  icon: Icon,
  iconClassName = "text-primary",
  children,
}: {
  icon: LucideIcon;
  iconClassName?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
      <Icon className={cn("h-4 w-4", iconClassName)} />
      {children}
    </label>
  );
}

/** Accessible on/off switch with its yes/no caption. */
export function Toggle({
  checked,
  onChange,
  caption,
  activeClassName = "bg-primary",
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
  caption: string;
  activeClassName?: string;
}) {
  return (
    <div className="flex items-center gap-3">
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
          checked ? activeClassName : "bg-muted",
        )}
      >
        <span
          className={cn(
            "pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform",
            checked ? "translate-x-4" : "translate-x-0",
          )}
        />
      </button>
      <span className="text-sm text-muted-foreground">{caption}</span>
    </div>
  );
}

/** Two-line option card used by the mode / distribution / injection pickers. */
export function OptionCard({
  selected,
  title,
  description,
  onClick,
}: {
  selected: boolean;
  title: string;
  description?: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "rounded-md border px-3 py-2 text-left transition-colors",
        selected
          ? "border-primary bg-primary/10 text-primary"
          : "border-border text-muted-foreground hover:bg-accent",
      )}
    >
      <div className="text-sm font-medium">{title}</div>
      {description && <div className="mt-0.5 text-xs opacity-70">{description}</div>}
    </button>
  );
}

export function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-0.5 text-sm font-medium text-foreground">{value}</p>
    </div>
  );
}

export const inputClass =
  "w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring";
