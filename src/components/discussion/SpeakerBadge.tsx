import { useTranslation } from "react-i18next";
import type { SpeakerRole } from "@/lib/types";
import { cn } from "@/lib/utils";

const ROLE_LABELS: Record<SpeakerRole, string> = {
  IArbitre: "IArbitre",
  GladIAteur: "GladIAteur",
  user: "roles.user",  // i18n key
};

const ROLE_COLORS: Record<SpeakerRole, string> = {
  IArbitre: "bg-amber-500/15 text-amber-500 border-amber-500/30",
  GladIAteur: "bg-primary/15 text-primary border-primary/30",
  user: "bg-green-500/15 text-green-500 border-green-500/30",
};

// Unique hue per speaker name
function nameToHue(name: string): number {
  if (!name) return 0;
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  return Math.abs(hash) % 360;
}

export function SpeakerBadge({
  name,
  role,
  active,
  emoji,
}: {
  name: string;
  role: SpeakerRole;
  active?: boolean;
  emoji?: string;
}) {
  const { t } = useTranslation();
  const hue = nameToHue(name);
  const roleLabel = role === "user" ? t(ROLE_LABELS[role]) : ROLE_LABELS[role];

  return (
    <div className="flex items-center gap-2">
      <div
        className={cn(
          "flex h-7 w-7 items-center justify-center rounded-full text-xs font-bold",
          emoji ? "bg-muted" : "text-white",
          active && "ring-2 ring-primary ring-offset-1 ring-offset-background",
        )}
        style={emoji ? undefined : { backgroundColor: `oklch(0.6 0.15 ${hue})` }}
      >
        {emoji || (name || "?").charAt(0).toUpperCase()}
      </div>
      <div className="flex items-center gap-1.5">
        <span className="text-sm font-medium text-foreground">{name}</span>
        <span
          className={cn(
            "rounded-full border px-1.5 py-0.5 text-[10px] font-medium",
            ROLE_COLORS[role],
          )}
        >
          {roleLabel}
        </span>
      </div>
    </div>
  );
}
