import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Home, MessageSquarePlus } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";

export default function SummaryPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const synthesis = useArenaStore((s) => s.synthesis);
  const currentTurn = useArenaStore((s) => s.currentTurn);
  const topic = useSetupStore((s) => s.topic);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const arenaReset = useArenaStore((s) => s.reset);
  const setupReset = useSetupStore((s) => s.reset);

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
          <div className="grid grid-cols-3 gap-4">
            <StatCard
              label={t("summary.topic")}
              value={topic || "-"}
              truncate
            />
            <StatCard
              label={t("summary.turns")}
              value={String(currentTurn)}
            />
            <StatCard
              label={t("summary.participants")}
              value={String(gladiateurs.length)}
            />
          </div>

          {/* Synthesis */}
          <div className="rounded-lg border border-border bg-card p-6">
            <h2 className="mb-4 text-lg font-semibold text-foreground">
              {t("summary.title")}
            </h2>
            <div className="prose prose-sm max-w-none text-sm text-foreground">
              <p className="whitespace-pre-wrap">
                {synthesis || t("summary.noSynthesis")}
              </p>
            </div>
          </div>

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
