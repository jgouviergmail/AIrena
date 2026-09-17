import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { History, Home, MessageSquarePlus } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { DiscussionReportView, type DiscussionReportData } from "@/components/report/DiscussionReportView";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import { describeActiveModel, useSettingsStore } from "@/stores/useSettingsStore";
import { getProfileEmoji, ROLE_EMOJIS } from "@/lib/profile-emoji";
import type { ParticipantInfo, SpeakerRole } from "@/lib/types";

const NAV_BUTTON = "flex items-center gap-2 rounded-lg border border-border bg-card px-5 py-2.5 text-sm font-medium text-foreground transition-colors hover:bg-accent";

export default function SummaryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const synthesis = useArenaStore((s) => s.synthesis);
  const argumentMapMarkdown = useArenaStore((s) => s.argumentMapMarkdown);
  const argumentMapMarkdownBySpeaker = useArenaStore((s) => s.argumentMapMarkdownBySpeaker);
  const argumentMap = useArenaStore((s) => s.argumentMap);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const messages = useArenaStore((s) => s.messages);
  const llmUsage = useArenaStore((s) => s.llmUsage);
  const documentContent = useArenaStore((s) => s.documentContent);
  const documentFormat = useArenaStore((s) => s.documentFormat);
  const buildReport = useArenaStore((s) => s.buildReport);
  const arenaReset = useArenaStore((s) => s.reset);
  const topic = useSetupStore((s) => s.topic);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arbitre = useSetupStore((s) => s.arbitre);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const setupReset = useSetupStore((s) => s.reset);
  const username = useSettingsStore((s) => s.settings.username);
  const modelName = useSettingsStore((s) => describeActiveModel(s.settings, [arbitre.model, ...gladiateurs.map((g) => g.model)]));

  const participants = useMemo<ParticipantInfo[]>(() => [
    { id: arbitre.id, name: arbitre.name, role: "IArbitre" as SpeakerRole, emoji: ROLE_EMOJIS.IArbitre },
    ...gladiateurs.map((g) => ({ id: g.id, name: g.name, role: "GladIAteur" as SpeakerRole, emoji: g.emoji ?? getProfileEmoji(g.name, g.systemPrompt) })),
    { id: "user", name: username, role: "user" as SpeakerRole, emoji: ROLE_EMOJIS.user },
  ], [arbitre, gladiateurs, username]);

  // Turns from messages as a robust fallback (currentTurn can be 0 on a very short run)
  const turns = currentTurn || (messages.length > 0 ? Math.max(...messages.map((m) => m.turnNumber ?? 0)) : 0);

  const data: DiscussionReportData = {
    topic,
    discussionMode,
    turns,
    modelName,
    participants,
    usage: llmUsage ? { provider: llmUsage.provider, model: llmUsage.model, total: llmUsage.total, calls: llmUsage.calls, estimatedCostUsd: llmUsage.estimatedCostUsd } : null,
    synthesis,
    messages,
    argumentMapMd: argumentMapMarkdown,
    argumentMapMdBySpeaker: argumentMapMarkdownBySpeaker,
    argumentMap,
    documentContent,
    documentFormat,
    report: buildReport(),
  };

  const leave = (path: string) => {
    arenaReset();
    setupReset();
    navigate(path);
  };

  return (
    <>
      <TopBar title={t("summary.title")} />
      <div className="flex-1 overflow-y-auto p-4 sm:p-6">
        <DiscussionReportView
          data={data}
          actions={
            <>
              <button onClick={() => leave("/")} className={NAV_BUTTON}>
                <Home className="h-4 w-4" />
                {t("summary.backHome")}
              </button>
              <button onClick={() => navigate("/history")} className={NAV_BUTTON}>
                <History className="h-4 w-4" />
                {t("summary.viewHistory")}
              </button>
              <button
                onClick={() => leave("/setup")}
                className="flex items-center gap-2 rounded-lg bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
              >
                <MessageSquarePlus className="h-4 w-4" />
                {t("summary.newDiscussion")}
              </button>
            </>
          }
        />
      </div>
    </>
  );
}
