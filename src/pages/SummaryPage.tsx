import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { History, Home, MessageSquarePlus } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { cn } from "@/lib/utils";
import type { ParticipantInfo, SpeakerRole } from "@/lib/types";

export default function SummaryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [tab, setTab] = useState<"synthesis" | "discussion">("synthesis");
  const synthesis = useArenaStore((s) => s.synthesis);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const messages = useArenaStore((s) => s.messages);
  const topic = useSetupStore((s) => s.topic);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);
  const username = useSettingsStore((s) => s.settings.username);

  // Compute turns from messages as robust fallback (currentTurn can be undefined)
  const turnsCompleted = currentTurn
    || (messages.length > 0
      ? Math.max(...messages.map((m) => m.turnNumber ?? 0))
      : 0);
  const arenaReset = useArenaStore((s) => s.reset);
  const setupReset = useSetupStore((s) => s.reset);

  const participants = useMemo<ParticipantInfo[]>(() => [
    { id: arbitre.id, name: arbitre.name, role: "IArbitre" as SpeakerRole, emoji: ROLE_EMOJIS.IArbitre },
    ...gladiateurs.map((g) => ({
      id: g.id,
      name: g.name,
      role: "GladIAteur" as SpeakerRole,
      emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt),
    })),
    { id: "user", name: username, role: "user" as SpeakerRole, emoji: ROLE_EMOJIS.user },
  ], [arbitre, gladiateurs, username]);

  const handleNewDiscussion = () => {
    arenaReset();
    setupReset();
    navigate("/setup");
  };

  const handleHome = () => {
    arenaReset();
    setupReset();
    navigate("/");
  };

  return (
    <>
      <TopBar title={t("summary.title")} />
      <div className="flex-1 overflow-y-auto p-6">
        <div className="mx-auto max-w-2xl space-y-6">
          {/* Stats */}
          <div className="space-y-3">
            <StatCard
              label={t("summary.topic")}
              value={topic || "-"}
            />
            <div className="grid grid-cols-2 gap-3">
              <StatCard
                label={t("summary.turns")}
                value={String(turnsCompleted)}
              />
              <StatCard
                label={t("summary.participants")}
                value={String(gladiateurs.length)}
              />
            </div>
          </div>

          {/* Tab toggle */}
          <div className="flex rounded-lg border border-border bg-card p-1">
            <button
              onClick={() => setTab("synthesis")}
              className={cn(
                "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                tab === "synthesis"
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t("summary.tabSynthesis")}
            </button>
            <button
              onClick={() => setTab("discussion")}
              className={cn(
                "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                tab === "discussion"
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t("summary.tabDiscussion")}
            </button>
          </div>

          {/* Tab content */}
          {tab === "synthesis" ? (
            <div className="rounded-lg border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold text-foreground">
                {t("summary.tabSynthesis")}
              </h2>
              <div className="prose prose-sm max-w-none text-sm text-foreground">
                <p className="whitespace-pre-wrap">
                  {synthesis || t("summary.noSynthesis")}
                </p>
              </div>
            </div>
          ) : (
            <ReadOnlyFeed messages={messages} participants={participants} />
          )}

          {/* Actions */}
          <div className="flex justify-center gap-4">
            <button
              onClick={handleHome}
              className="flex items-center gap-2 rounded-lg border border-border bg-card px-5 py-2.5 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            >
              <Home className="h-4 w-4" />
              {t("summary.backHome")}
            </button>
            <button
              onClick={() => navigate("/history")}
              className="flex items-center gap-2 rounded-lg border border-border bg-card px-5 py-2.5 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            >
              <History className="h-4 w-4" />
              {t("summary.viewHistory")}
            </button>
            <button
              onClick={handleNewDiscussion}
              className="flex items-center gap-2 rounded-lg bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
            >
              <MessageSquarePlus className="h-4 w-4" />
              {t("summary.newDiscussion")}
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

function StatCard({
  label,
  value,
  truncate,
}: {
  label: string;
  value: string;
  truncate?: boolean;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-3 text-center">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p
        className={`mt-1 text-sm font-semibold text-foreground ${truncate ? "truncate" : ""}`}
        title={truncate ? value : undefined}
      >
        {value}
      </p>
    </div>
  );
}
