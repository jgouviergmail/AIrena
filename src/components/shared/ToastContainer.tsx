import { useState } from "react";
import { AlertCircle, CheckCircle, ChevronDown, ChevronUp, Info, X, AlertTriangle } from "lucide-react";
import { useToastStore, type Toast, type ToastLevel } from "@/stores/useToastStore";
import { cn } from "@/lib/utils";

const LEVEL_STYLES: Record<ToastLevel, { bg: string; border: string; icon: string }> = {
  success: { bg: "bg-green-500/10", border: "border-green-500/30", icon: "text-green-500" },
  error: { bg: "bg-destructive/10", border: "border-destructive/30", icon: "text-destructive" },
  warning: { bg: "bg-yellow-500/10", border: "border-yellow-500/30", icon: "text-yellow-500" },
  info: { bg: "bg-blue-500/10", border: "border-blue-500/30", icon: "text-blue-500" },
};

const LEVEL_ICONS: Record<ToastLevel, typeof AlertCircle> = {
  success: CheckCircle,
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
};

function ToastItem({ toast: t, onClose }: { toast: Toast; onClose: () => void }) {
  const [expanded, setExpanded] = useState(false);
  const styles = LEVEL_STYLES[t.level];
  const Icon = LEVEL_ICONS[t.level];

  return (
    <div
      className={cn(
        "pointer-events-auto flex w-96 max-w-[calc(100vw-2rem)] flex-col gap-1 rounded-lg border px-4 py-3 shadow-lg backdrop-blur-sm animate-in slide-in-from-right-5 fade-in duration-200",
        styles.bg,
        styles.border,
      )}
    >
      <div className="flex items-start gap-3">
        <Icon className={cn("mt-0.5 h-4 w-4 shrink-0", styles.icon)} />
        <p className="flex-1 text-sm font-medium text-foreground">{t.message}</p>
        <button
          onClick={onClose}
          className="shrink-0 rounded p-0.5 text-muted-foreground hover:text-foreground"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
      {t.detail && (
        <>
          <button
            onClick={() => setExpanded(!expanded)}
            className="flex items-center gap-1 self-start pl-7 text-xs text-muted-foreground hover:text-foreground"
          >
            {expanded ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
            {expanded ? "Masquer" : "Détail"}
          </button>
          {expanded && (
            <pre className="ml-7 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-background/50 p-2 text-xs text-muted-foreground">
              {t.detail}
            </pre>
          )}
        </>
      )}
    </div>
  );
}

export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);
  const removeToast = useToastStore((s) => s.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div className="pointer-events-none fixed right-4 top-4 z-50 flex flex-col gap-2">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} onClose={() => removeToast(t.id)} />
      ))}
    </div>
  );
}
