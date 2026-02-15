import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { History, KeyRound, MessageSquarePlus, Settings, Swords } from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { useArenaStore } from "@/stores/useArenaStore";
import { useSetupStore } from "@/stores/useSetupStore";
import type { LicenseStatus } from "@/lib/types";
import * as api from "@/lib/tauri-api";

export default function HomePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const status = useArenaStore((s) => s.status);
  const [licenseStatus, setLicenseStatus] = useState<LicenseStatus | null>(null);

  useEffect(() => {
    api.checkLicenseStatus().then(setLicenseStatus).catch(() => {});
  }, []);

  const isDiscussionActive =
    status === "running" || status === "paused" || status === "synthesizing";

  const handleNewDiscussion = () => {
    useArenaStore.getState().reset();
    useSetupStore.getState().reset();
    navigate("/setup");
  };

  return (
    <>
      <TopBar title={t("app.name")} />
      <div className="flex flex-1 flex-col items-center justify-center gap-8 p-8">
        <div className="flex flex-col items-center gap-3">
          <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-primary">
            <Swords className="h-8 w-8 text-primary-foreground" />
          </div>
          <h1 className="text-3xl font-bold text-foreground">
            {t("home.welcome")}
          </h1>
          <p className="max-w-md text-center text-muted-foreground">
            {t("home.description")}
          </p>
        </div>

        {licenseStatus && !licenseStatus.valid && (
          <div className="flex items-center gap-2 rounded-lg border border-destructive/20 bg-destructive/5 px-4 py-2.5 text-sm text-destructive">
            <KeyRound className="h-4 w-4 shrink-0" />
            <span>
              {licenseStatus.error === "No license key configured"
                ? t("home.licenseNeeded")
                : t("home.licenseExpired")}
            </span>
          </div>
        )}

        <div className="flex gap-4">
          {isDiscussionActive ? (
            <button
              onClick={() => navigate("/arena")}
              className="flex items-center gap-2 rounded-lg bg-primary px-6 py-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
            >
              <Swords className="h-4 w-4" />
              {t("home.resumeDiscussion")}
            </button>
          ) : (
            <button
              onClick={handleNewDiscussion}
              disabled={licenseStatus?.valid !== true}
              className="flex items-center gap-2 rounded-lg bg-primary px-6 py-3 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-50"
            >
              <MessageSquarePlus className="h-4 w-4" />
              {t("home.startDiscussion")}
            </button>
          )}
          <button
            onClick={() => navigate("/history")}
            className="flex items-center gap-2 rounded-lg border border-border bg-card px-6 py-3 text-sm font-medium text-foreground transition-colors hover:bg-accent"
          >
            <History className="h-4 w-4" />
            {t("home.history")}
          </button>
          <button
            onClick={() => navigate("/settings")}
            className="flex items-center gap-2 rounded-lg border border-border bg-card px-6 py-3 text-sm font-medium text-foreground transition-colors hover:bg-accent"
          >
            <Settings className="h-4 w-4" />
            {t("home.goToSettings")}
          </button>
        </div>
      </div>
    </>
  );
}
