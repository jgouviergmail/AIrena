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
      <div className="relative flex flex-1 flex-col items-center gap-8 overflow-hidden pt-12">
        <img
          src="/AIrena.jpg"
          alt=""
          className="pointer-events-none absolute inset-0 h-full w-full object-cover"
        />
        <div className="absolute inset-0 bg-black/60" />

        <div className="relative z-10 flex flex-col items-center gap-4 px-8">
          <h1 className="max-w-2xl whitespace-pre-line text-center text-3xl font-semibold italic leading-snug text-white drop-shadow-lg">
            {t("home.welcome").split(". ").join(".\n")}
          </h1>
          <p className="max-w-lg whitespace-pre-line text-center text-lg text-white/80">
            {t("home.description").split(". ").join(".\n")}
          </p>
        </div>

        {licenseStatus && !licenseStatus.valid && (
          <div className="relative z-10 flex items-center gap-2 rounded-lg border border-red-400/30 bg-red-950/60 px-4 py-2.5 text-sm text-red-300 backdrop-blur-sm">
            <KeyRound className="h-4 w-4 shrink-0" />
            <span>
              {licenseStatus.error === "No license key configured"
                ? t("home.licenseNeeded")
                : t("home.licenseExpired")}
            </span>
          </div>
        )}

        <div className="relative z-10 mt-auto flex w-64 flex-col gap-3" style={{ marginBottom: "15%" }}>
          {isDiscussionActive ? (
            <button
              onClick={() => navigate("/arena")}
              className="flex w-full items-center justify-center gap-2 rounded-lg bg-primary px-6 py-3 text-sm font-medium text-primary-foreground shadow-lg transition-colors hover:bg-primary/90"
            >
              <Swords className="h-4 w-4" />
              {t("home.resumeDiscussion")}
            </button>
          ) : (
            <button
              onClick={handleNewDiscussion}
              disabled={licenseStatus?.valid !== true}
              className="flex w-full items-center justify-center gap-2 rounded-lg bg-primary px-6 py-3 text-sm font-medium text-primary-foreground shadow-lg transition-colors hover:bg-primary/90 disabled:opacity-50"
            >
              <MessageSquarePlus className="h-4 w-4" />
              {t("home.startDiscussion")}
            </button>
          )}
          <button
            onClick={() => navigate("/history")}
            className="flex w-full items-center justify-center gap-2 rounded-lg border border-white/20 bg-white/10 px-6 py-3 text-sm font-medium text-white backdrop-blur-sm transition-colors hover:bg-white/20"
          >
            <History className="h-4 w-4" />
            {t("home.history")}
          </button>
          <button
            onClick={() => navigate("/settings")}
            className="flex w-full items-center justify-center gap-2 rounded-lg border border-white/20 bg-white/10 px-6 py-3 text-sm font-medium text-white backdrop-blur-sm transition-colors hover:bg-white/20"
          >
            <Settings className="h-4 w-4" />
            {t("home.goToSettings")}
          </button>
        </div>
      </div>
    </>
  );
}
