import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Database, Globe, Layers, Loader2, Server, Wifi, WifiOff, Zap } from "lucide-react";
import { VramIndicator } from "@/components/setup/VramIndicator";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import { buttonClass, Explainer, Field, inputClass, Section, StatusPill } from "./SettingsPrimitives";

const NUM_CTX_MIN = 2048;
const NUM_CTX_FALLBACK_MAX = 131072;
/** Allow going above the VRAM recommendation, within reason. */
const NUM_CTX_OVER_RECOMMENDED = 1.5;

const isEmbeddingModel = (name: string) => /embed|nomic/i.test(name);

/**
 * Ollama connection, chat model, embedding model and context window.
 * `chatEnabled` is false when DeepSeek serves the discussion: Ollama is then
 * only relevant for RAG embeddings, so the chat model / context controls hide.
 */
export function OllamaSettings({ chatEnabled }: { chatEnabled: boolean }) {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const models = useSettingsStore((s) => s.models);
  const ollamaConnected = useSettingsStore((s) => s.ollamaConnected);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const checkOllama = useSettingsStore((s) => s.checkOllama);
  const preloadModel = useSettingsStore((s) => s.preloadModel);
  const preloading = useSettingsStore((s) => s.preloading);
  const preloadDone = useSettingsStore((s) => s.preloadDone);
  const preloadError = useSettingsStore((s) => s.preloadError);
  const modelBudgetInfo = useSettingsStore((s) => s.modelBudgetInfo);
  const modelBudgetLoading = useSettingsStore((s) => s.modelBudgetLoading);
  const fetchModelBudgetInfo = useSettingsStore((s) => s.fetchModelBudgetInfo);
  const initializingOllama = useSettingsStore((s) => s.initializingOllama);
  const ollamaInitialized = useSettingsStore((s) => s.ollamaInitialized);
  const [checking, setChecking] = useState(false);

  // Skip Ollama check if initialization is already in progress
  useEffect(() => {
    if (!initializingOllama) checkOllama();
  }, [checkOllama, initializingOllama]);

  // Track whether model was changed by user (vs hydration/mount)
  const userChangedModelRef = useRef(false);

  // Fetch model budget info (VRAM + architecture) when model changes
  useEffect(() => {
    if (chatEnabled && settings.ollamaModel && !initializingOllama) {
      fetchModelBudgetInfo(settings.ollamaModel, userChangedModelRef.current);
      userChangedModelRef.current = false;
    }
  }, [chatEnabled, settings.ollamaModel, fetchModelBudgetInfo, initializingOllama]);

  // numCtx local state — commit on blur/Enter to avoid clamping on every keystroke
  const [numCtxInput, setNumCtxInput] = useState(String(settings.numCtx));
  useEffect(() => setNumCtxInput(String(settings.numCtx)), [settings.numCtx]);

  const numCtxMax = modelBudgetInfo?.recommendedNumCtx
    ? Math.round(modelBudgetInfo.recommendedNumCtx * NUM_CTX_OVER_RECOMMENDED)
    : NUM_CTX_FALLBACK_MAX;

  const commitNumCtx = () => {
    const parsed = parseInt(numCtxInput) || NUM_CTX_MIN;
    const clamped = Math.max(NUM_CTX_MIN, Math.min(numCtxMax, parsed));
    setNumCtxInput(String(clamped));
    const prev = settings.numCtx;
    updateSettings({ numCtx: clamped });
    // Reload the model with the new num_ctx so Ollama reallocates the KV cache.
    if (clamped !== prev && settings.ollamaModel) preloadModel(settings.ollamaModel);
  };

  const handleCheckOllama = async () => {
    setChecking(true);
    try {
      await checkOllama();
    } finally {
      setChecking(false);
    }
  };

  const refreshVram = async () => {
    if (!settings.ollamaModel) return;
    try {
      await saveSettings();
      fetchModelBudgetInfo(settings.ollamaModel, false);
    } catch (err: unknown) {
      toast.error(t("settings.saveError"), extractErrorMessage(err));
    }
  };

  const hasModels = ollamaConnected && models.length > 0;

  return (
    <Section title={t("settings.ollama")} icon={Server}>
      {!chatEnabled && <Explainer>{t("settings.ollamaEmbeddingsOnly")}</Explainer>}

      <Field label={<><Globe className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.ollamaUrl")}</>}>
        <div className="flex flex-wrap gap-2">
          <input
            type="text"
            value={settings.ollamaUrl}
            onChange={(e) => updateSettings({ ollamaUrl: e.target.value })}
            className={`${inputClass} min-w-48 flex-1`}
          />
          <button onClick={handleCheckOllama} disabled={checking} className={buttonClass}>
            {checking ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : ollamaConnected ? (
              <Wifi className="h-3.5 w-3.5 text-green-500" />
            ) : (
              <WifiOff className="h-3.5 w-3.5 text-destructive" />
            )}
            {t("settings.ollamaCheck")}
          </button>
        </div>
      </Field>

      <Field label={t("settings.ollamaStatus")}>
        <StatusPill tone={ollamaConnected ? "ok" : "error"}>
          {ollamaConnected ? t("settings.ollamaConnected") : t("settings.ollamaDisconnected")}
        </StatusPill>
      </Field>

      {chatEnabled && hasModels && (
        <Field label={<><Server className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.ollamaModel")}</>}>
          <Explainer>{t("settings.thinkModeExplanation")}</Explainer>
          <select
            value={settings.ollamaModel}
            onChange={(e) => {
              const model = e.target.value;
              userChangedModelRef.current = true;
              updateSettings({ ollamaModel: model });
              if (model) preloadModel(model);
            }}
            disabled={preloading}
            className={inputClass}
          >
            <option value="">--</option>
            {models.map((m) => (
              <option key={m.name} value={m.name}>
                {m.name} ({(m.size / 1e9).toFixed(1)} GB)
              </option>
            ))}
          </select>
          {(preloading || preloadDone || preloadError) && (
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-muted-foreground">{t("settings.ollamaStatus")}</span>
              {preloading ? (
                <StatusPill tone="muted" icon={<Loader2 className="h-3 w-3 animate-spin" />}>{t("settings.modelPreloading")}</StatusPill>
              ) : preloadDone ? (
                <StatusPill tone="ok">{t("settings.modelPreloaded")}</StatusPill>
              ) : (
                <StatusPill tone="error">{t("settings.modelPreloadError")}</StatusPill>
              )}
            </div>
          )}
          {preloadDone && !preloading && modelBudgetInfo && (
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-muted-foreground">{t("settings.thinkModeLabel")}</span>
              <StatusPill tone={modelBudgetInfo.supportsThink ? "ok" : "warn"}>
                {modelBudgetInfo.supportsThink ? t("settings.thinkAvailable") : t("settings.thinkUnavailable")}
              </StatusPill>
            </div>
          )}
        </Field>
      )}
      {chatEnabled && hasModels && <div className="border-t border-border" />}

      {hasModels && (
        <Field label={<><Database className="mr-1 inline h-3.5 w-3.5 text-purple-500" />{t("settings.embeddingModel")}</>}>
          <Explainer>{t("settings.embeddingModelDesc")}</Explainer>
          <select
            value={settings.embeddingModel}
            onChange={async (e) => {
              updateSettings({ embeddingModel: e.target.value });
              try {
                // Force save so backend reads the new embedding model, then refresh VRAM info
                await saveSettings();
                if (chatEnabled && settings.ollamaModel) fetchModelBudgetInfo(settings.ollamaModel, false);
              } catch (err: unknown) {
                toast.error(t("settings.saveError"), extractErrorMessage(err));
              }
            }}
            className={inputClass}
          >
            <option value="">{chatEnabled ? t("settings.embeddingModelAuto") : t("settings.embeddingModelNone")}</option>
            {models.filter((m) => isEmbeddingModel(m.name)).map((m) => (
              <option key={m.name} value={m.name}>{m.name} ({(m.size / 1e9).toFixed(1)} GB)</option>
            ))}
            {models.some((m) => isEmbeddingModel(m.name)) && <option disabled>───</option>}
            {models.filter((m) => !isEmbeddingModel(m.name)).map((m) => (
              <option key={m.name} value={m.name}>{m.name} ({(m.size / 1e9).toFixed(1)} GB)</option>
            ))}
          </select>
        </Field>
      )}
      {hasModels && <div className="border-t border-border" />}

      {initializingOllama && !ollamaInitialized && (
        <div className="flex items-center gap-2 rounded-md border border-primary/30 bg-primary/5 px-3 py-2 text-xs text-primary">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          {t("settings.ollamaInitializing")}
        </div>
      )}

      {/* Context window size (numCtx) + VRAM indicator */}
      {chatEnabled && ollamaConnected && settings.ollamaModel && (
        <Field label={<><Layers className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.numCtxLabel")}</>}>
          <Explainer>{t("settings.numCtxDesc")}</Explainer>
          <VramIndicator
            info={modelBudgetInfo}
            loading={modelBudgetLoading || initializingOllama}
            hideThinkIndicator
            hideWarnings
            onRefresh={refreshVram}
          />
          <div className="flex flex-wrap items-center gap-3">
            <input
              type="number"
              min={NUM_CTX_MIN}
              max={numCtxMax}
              value={numCtxInput}
              onChange={(e) => setNumCtxInput(e.target.value)}
              onBlur={commitNumCtx}
              onKeyDown={(e) => e.key === "Enter" && commitNumCtx()}
              className={`${inputClass} w-32 font-mono`}
            />
            <span className="text-sm text-muted-foreground">tokens</span>
            {modelBudgetInfo?.recommendedNumCtx && (
              <button
                onClick={() => {
                  const rec = modelBudgetInfo.recommendedNumCtx!;
                  updateSettings({ numCtx: rec });
                  if (rec !== settings.numCtx && settings.ollamaModel) preloadModel(settings.ollamaModel);
                }}
                title={t("setup.vramAutoFillTooltip", { value: modelBudgetInfo.recommendedNumCtx.toLocaleString() })}
                className={cn(
                  "flex items-center gap-1 rounded-md border border-primary/30 bg-primary/5 px-2 py-1 text-xs font-medium text-primary",
                  "transition-colors hover:bg-primary/10",
                )}
              >
                <Zap className="h-3 w-3" />
                AUTO — {modelBudgetInfo.recommendedNumCtx.toLocaleString()}
              </button>
            )}
          </div>
          {modelBudgetInfo && modelBudgetInfo.warnings.length > 0 && (
            <div className="space-y-0.5">
              {modelBudgetInfo.warnings.map((w, i) => (
                <p key={i} className="text-xs text-amber-500">{w}</p>
              ))}
            </div>
          )}
        </Field>
      )}

      {!ollamaConnected && (
        <div className="rounded-lg border border-border bg-muted/50 p-4">
          <h4 className="mb-1 text-sm font-medium text-foreground">{t("settings.ollamaGuide")}</h4>
          <p className="text-sm text-muted-foreground">{t("settings.ollamaGuideText")}</p>
          <ol className="mt-2 list-inside list-decimal space-y-1 text-sm text-muted-foreground">
            <li>{t("settings.ollamaGuideStep1")} <span className="font-mono text-primary">https://ollama.com</span></li>
            <li>{t("settings.ollamaGuideStep2")}</li>
            <li>{t("settings.ollamaGuideStep3")} <span className="font-mono text-primary">ollama pull llama3.2</span></li>
            <li>{t("settings.ollamaGuideStep4")}</li>
          </ol>
        </div>
      )}
    </Section>
  );
}
