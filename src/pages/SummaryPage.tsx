import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Download, History, Home, MessageSquarePlus, Bot, Network, Users, Repeat } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { ReadOnlyFeed } from "@/components/discussion/ReadOnlyFeed";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import { SimpleMd } from "@/components/shared/SimpleMd";
import { MarkmapViewer } from "@/components/mindmap/MarkmapViewer";
import type { MarkmapViewerHandle } from "@/components/mindmap/MarkmapViewer";
import { cn } from "@/lib/utils";
import { downloadTextFile } from "@/lib/tauri-api";
import type { ParticipantInfo, SpeakerRole } from "@/lib/types";

export default function SummaryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [tab, setTab] = useState<"synthesis" | "discussion" | "argumentMap">("synthesis");
  const markmapRef = useRef<MarkmapViewerHandle>(null);
  const synthesis = useArenaStore((s) => s.synthesis);
  const argumentMapMarkdown = useArenaStore((s) => s.argumentMapMarkdown);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const messages = useArenaStore((s) => s.messages);
  const topic = useSetupStore((s) => s.topic);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);
  const username = useSettingsStore((s) => s.settings.username);
  const modelName = useSettingsStore((s) => s.settings.ollamaModel);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const documentContent = useArenaStore((s) => s.documentContent);
  const documentFormat = useArenaStore((s) => s.documentFormat);

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
            {/* Topic — prominent */}
            <div className="rounded-xl border border-border bg-card p-4">
              <p className="text-xs text-muted-foreground">{t("summary.topic")}</p>
              <p className="mt-1 text-base font-semibold text-foreground">{topic || "-"}</p>
              {discussionMode !== "debate" && (
                <span className="mt-1 inline-block rounded bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                  {t(`setup.mode_${discussionMode}`)}
                </span>
              )}
            </div>

            <div className="grid grid-cols-3 gap-3">
              <StatCard
                label={t("summary.turns")}
                value={String(turnsCompleted)}
                icon={<Repeat className="h-3.5 w-3.5 text-muted-foreground" />}
              />
              <StatCard
                label={t("summary.model")}
                value={modelName || "-"}
                icon={<Bot className="h-3.5 w-3.5 text-muted-foreground" />}
                truncate
              />
              <StatCard
                label={t("summary.participantsList")}
                value={String(gladiateurs.length)}
                icon={<Users className="h-3.5 w-3.5 text-muted-foreground" />}
              />
            </div>

            {/* Participants with emojis */}
            <div className="rounded-xl border border-border bg-card p-4">
              <p className="mb-2 text-xs text-muted-foreground">{t("summary.participantsList")}</p>
              <div className="flex flex-wrap gap-3">
                {participants.map((p) => (
                  <div key={p.id} className="flex items-center gap-1.5 text-sm text-foreground">
                    <span>{p.emoji}</span>
                    <span className="font-medium">{p.name}</span>
                    {p.role === "IArbitre" && (
                      <span className="rounded bg-primary/10 px-1 text-[9px] font-medium text-primary">IArbitre</span>
                    )}
                  </div>
                ))}
              </div>
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
            {argumentMapMarkdown && (
              <button
                onClick={() => setTab("argumentMap")}
                className={cn(
                  "flex-1 rounded-md px-4 py-2 text-sm font-medium transition-colors",
                  tab === "argumentMap"
                    ? "bg-primary text-primary-foreground"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                <Network className="mr-1.5 inline h-3.5 w-3.5" />
                {t("summary.tabArgumentMap")}
              </button>
            )}
          </div>

          {/* Tab content */}
          {tab === "synthesis" ? (
            <div className="rounded-xl border border-border bg-card p-6">
              <h2 className="mb-4 text-lg font-semibold text-foreground">
                {t("summary.tabSynthesis")}
              </h2>
              {synthesis ? (
                <SimpleMd text={synthesis} />
              ) : (
                <p className="text-sm text-muted-foreground">{t("summary.noSynthesis")}</p>
              )}
            </div>
          ) : tab === "discussion" ? (
            <ReadOnlyFeed messages={messages} participants={participants} />
          ) : (
            <div className="rounded-xl border border-border bg-card p-2" style={{ height: 400 }}>
              <MarkmapViewer ref={markmapRef} markdown={argumentMapMarkdown} />
            </div>
          )}

          {/* Document download */}
          {documentContent && documentFormat !== "none" && (
            <div className="flex justify-center">
              <button
                onClick={() => downloadTextFile(documentContent, `airena-document.${documentFormat}`).catch((e) => console.error("Download failed:", e))}
                className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
              >
                <Download className="h-4 w-4" />
                {t("summary.downloadDocument")} (.{documentFormat})
              </button>
            </div>
          )}

          {/* Argument map download */}
          {argumentMapMarkdown && (
            <div className="flex justify-center gap-3">
              <button
                onClick={() => downloadTextFile(argumentMapMarkdown, "airena-argument-map.md").catch((e) => console.error("Download failed:", e))}
                className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
              >
                <Download className="h-4 w-4" />
                {t("summary.downloadArgumentMap")} (.md)
              </button>
              {tab === "argumentMap" && (
                <button
                  onClick={() => {
                    const svgHtml = markmapRef.current?.getSvgHtml();
                    if (svgHtml) {
                      downloadTextFile(svgHtml, "airena-argument-map.svg").catch((e) => console.error("SVG download failed:", e));
                    }
                  }}
                  className="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/5 px-5 py-2.5 text-sm font-medium text-primary transition-colors hover:bg-primary/10"
                >
                  <Download className="h-4 w-4" />
                  {t("summary.downloadArgumentMapSvg")} (.svg)
                </button>
              )}
            </div>
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
  icon,
  truncate,
}: {
  label: string;
  value: string;
  icon?: React.ReactNode;
  truncate?: boolean;
}) {
  return (
    <div className="rounded-xl border border-border bg-card p-3 text-center">
      <div className="flex items-center justify-center gap-1">
        {icon}
        <p className="text-xs text-muted-foreground">{label}</p>
      </div>
      <p
        className={`mt-1 text-sm font-semibold text-foreground ${truncate ? "truncate" : ""}`}
        title={truncate ? value : undefined}
      >
        {value}
      </p>
    </div>
  );
}

