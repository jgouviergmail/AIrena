import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Eye, EyeOff, KeyRound, Loader2, Plug, RefreshCw, Server } from "lucide-react";
import { useSettingsStore, parseModelList, serialiseModelList } from "@/stores/useSettingsStore";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";
import * as api from "@/lib/tauri-api";
import { buttonClass, Explainer, Field, inputClass, Section, secretInputClass, StatusPill } from "./SettingsPrimitives";

/**
 * Any OpenAI-compatible server (LM Studio, vLLM, llama.cpp, OpenRouter…):
 * base URL, optional key, model. The server's catalogue is fetched when it
 * publishes one; otherwise the models are typed by hand. Nothing is billed.
 */
export function OpenAiCompatSettings() {
  const { t } = useTranslation();
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const llmConstants = useSettingsStore((s) => s.llmConstants);
  const loadLlmConstants = useSettingsStore((s) => s.loadLlmConstants);
  const [showKey, setShowKey] = useState(false);
  const [catalogue, setCatalogue] = useState<string[] | null>(null);
  const [loadingModels, setLoadingModels] = useState(false);
  const [testing, setTesting] = useState(false);
  const [tested, setTested] = useState<"ok" | "failed" | null>(null);

  useEffect(() => { loadLlmConstants(); }, [loadLlmConstants]);

  const manual = parseModelList(settings.openaiCompatModels);
  const known = catalogue && catalogue.length > 0 ? catalogue : manual;

  const refreshModels = async () => {
    setLoadingModels(true);
    try {
      const res = await api.listOpenAiCompatModels(settings.openaiCompatBaseUrl, settings.openaiCompatApiKey);
      setCatalogue(res.models);
      if (res.models.length === 0) toast.info(t("settings.openaiCompatNoCatalogue"));
      else if (!settings.openaiCompatModel) updateSettings({ openaiCompatModel: res.models[0] });
    } catch (e: unknown) {
      setCatalogue(null);
      toast.error(t("settings.openaiCompatModelsError"), extractErrorMessage(e));
    } finally {
      setLoadingModels(false);
    }
  };

  const testConnection = async () => {
    setTesting(true);
    setTested(null);
    try {
      await api.validateOpenAiCompat(settings.openaiCompatBaseUrl, settings.openaiCompatApiKey, settings.openaiCompatModel);
      setTested("ok");
      toast.success(t("settings.openaiCompatTestOk"));
    } catch (e: unknown) {
      setTested("failed");
      toast.error(t("settings.openaiCompatTestFailed"), extractErrorMessage(e));
    } finally {
      setTesting(false);
    }
  };

  const addManualModel = (name: string) => {
    const trimmed = name.trim();
    if (!trimmed || manual.includes(trimmed)) return;
    updateSettings({ openaiCompatModels: serialiseModelList([...manual, trimmed]), openaiCompatModel: settings.openaiCompatModel || trimmed });
  };

  return (
    <Section title={t("settings.openaiCompat")} icon={Server}>
      <Explainer>{t("settings.openaiCompatDesc")}</Explainer>

      <Field label={t("settings.openaiCompatBaseUrl")}>
        <input
          type="url"
          value={settings.openaiCompatBaseUrl}
          onChange={(e) => { setTested(null); updateSettings({ openaiCompatBaseUrl: e.target.value }); }}
          placeholder={llmConstants?.openaiCompatDefaultBaseUrl ?? "http://localhost:1234/v1"}
          className={inputClass}
          spellCheck={false}
        />
      </Field>

      <Field label={<><KeyRound className="mr-1 inline h-3.5 w-3.5 text-primary" />{t("settings.openaiCompatApiKey")}</>}>
        <Explainer>{t("settings.openaiCompatApiKeyDesc")}</Explainer>
        <div className="relative">
          <input
            type={showKey ? "text" : "password"}
            value={settings.openaiCompatApiKey}
            onChange={(e) => { setTested(null); updateSettings({ openaiCompatApiKey: e.target.value }); }}
            className={secretInputClass}
            autoComplete="off"
            spellCheck={false}
          />
          <button
            type="button"
            onClick={() => setShowKey(!showKey)}
            className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            aria-label={showKey ? t("settings.hideKey") : t("settings.showKey")}
          >
            {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
          </button>
        </div>
      </Field>

      <Field label={t("settings.openaiCompatModel")}>
        <div className="flex flex-wrap items-center gap-2">
          <input
            type="text"
            list="openai-compat-models"
            value={settings.openaiCompatModel}
            onChange={(e) => { setTested(null); updateSettings({ openaiCompatModel: e.target.value }); }}
            onBlur={(e) => addManualModel(e.target.value)}
            placeholder={t("settings.openaiCompatModelPlaceholder")}
            className={`${inputClass} min-w-0 flex-1`}
            spellCheck={false}
          />
          <datalist id="openai-compat-models">
            {known.map((m) => <option key={m} value={m} />)}
          </datalist>
          <button type="button" onClick={refreshModels} disabled={loadingModels || !settings.openaiCompatBaseUrl.trim()} className={buttonClass} title={t("settings.openaiCompatRefreshModels")}>
            {loadingModels ? <Loader2 className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}
            {t("settings.openaiCompatRefreshModels")}
          </button>
        </div>
        {catalogue && catalogue.length > 0 && (
          <p className="text-xs text-muted-foreground">{t("settings.openaiCompatCatalogue", { count: catalogue.length })}</p>
        )}
        {manual.length > 0 && (
          <p className="text-xs text-muted-foreground">{t("settings.openaiCompatManualList")}: {manual.join(", ")}</p>
        )}
      </Field>

      <div className="flex flex-wrap items-center gap-2">
        <button type="button" onClick={testConnection} disabled={testing || !settings.openaiCompatBaseUrl.trim() || !settings.openaiCompatModel.trim()} className={buttonClass}>
          {testing ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plug className="h-4 w-4" />}
          {t("settings.openaiCompatTest")}
        </button>
        {tested === "ok" && <StatusPill tone="ok">{t("settings.openaiCompatTestOk")}</StatusPill>}
        {tested === "failed" && <StatusPill tone="error">{t("settings.openaiCompatTestFailed")}</StatusPill>}
      </div>
      <p className="text-xs text-muted-foreground">{t("settings.openaiCompatNote")}</p>
    </Section>
  );
}
