import { useTranslation } from "react-i18next";
import { Cpu, Loader2, RefreshCw } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ModelBudgetInfo } from "@/lib/types";

interface Props {
  info: ModelBudgetInfo | null;
  loading: boolean;
  onRefresh?: () => void;
}

export function VramIndicator({ info, loading, onRefresh }: Props) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Loader2 className="h-3 w-3 animate-spin" />
        {t("settings.modelPreloading")}
      </div>
    );
  }

  if (!info) return null;

  const { vram, arch } = info;

  return (
    <div className="space-y-1">
      {/* GPU info + refresh button */}
      {vram ? (
        <div className="space-y-0.5">
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Cpu className="h-3 w-3 shrink-0" />
            <span className="font-medium">{vram.gpuName}</span>
            {onRefresh && (
              <button
                onClick={onRefresh}
                title={t("setup.vramRefreshTooltip")}
                className={cn(
                  "ml-1 rounded p-0.5 text-muted-foreground/60 transition-colors",
                  "hover:bg-accent hover:text-foreground",
                )}
              >
                <RefreshCw className="h-3 w-3" />
              </button>
            )}
          </div>

          {/* Architecture info (moved here from below) */}
          {arch && (
            <div className="text-xs text-muted-foreground/70">
              {t("setup.vramModelLabel")}: {arch.family} — {arch.blockCount}L / {arch.headCountKv} KV heads / {arch.quantization}
              {" — "}
              {(arch.kvBytesPerToken / 1024).toFixed(1)} KiB/token
            </div>
          )}

          {/* VRAM lines — consistent style */}
          <div className="text-xs text-muted-foreground/70">
            {t("setup.vramTotalLabel")}: {vram.totalMb.toLocaleString()} {t("setup.vramUnit")}
          </div>
          {info.ollamaVramMb != null && (
            <div className="text-xs text-muted-foreground/70">
              {t("setup.vramOllamaLabel")}: {info.ollamaVramMb.toLocaleString()} {t("setup.vramUnit")}
            </div>
          )}
          <div className="text-xs text-muted-foreground/70">
            {t("setup.vramFreeLabel")}: {vram.freeMb.toLocaleString()} {t("setup.vramUnit")}
          </div>
        </div>
      ) : (
        <div className="flex items-center gap-1.5 text-xs text-amber-500">
          <Cpu className="h-3 w-3 shrink-0" />
          {t("setup.vramDetectionFailed")}
          {onRefresh && (
            <button
              onClick={onRefresh}
              title={t("setup.vramRefreshTooltip")}
              className="ml-1 rounded p-0.5 text-amber-500/60 transition-colors hover:bg-accent hover:text-foreground"
            >
              <RefreshCw className="h-3 w-3" />
            </button>
          )}
        </div>
      )}

      {/* Warnings */}
      {info.warnings.length > 0 && (
        <div className="space-y-0.5">
          {info.warnings.map((w, i) => (
            <p key={i} className="text-xs text-amber-500">{w}</p>
          ))}
        </div>
      )}
    </div>
  );
}
