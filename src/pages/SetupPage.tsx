import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Check,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Clock,
  Database,
  FileText,
  Globe,
  GripVertical,
  Network,
  Heart,
  Info,
  Loader2,
  MessageSquare,
  Play,
  Plus,
  Repeat,
  RotateCcw,
  Save,
  Shuffle,
  Sliders,
  Tag,
  Trash2,
  Upload,
  UserCircle,
  Users,
  X,
} from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { LlmParamsForm } from "@/components/setup/LlmParamsForm";
import { PersonaEditor } from "@/components/setup/PersonaEditor";
import { EmojiPicker } from "@/components/setup/EmojiPicker";
import { getProfileEmoji } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useArenaStore } from "@/stores/useArenaStore";
import { cn } from "@/lib/utils";
import { DEFAULT_LLM_PARAMS } from "@/lib/types";
import type { DiscussionMode, DocumentFormat, GladIAteurConfig, PredefinedProfile } from "@/lib/types";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { toast } from "@/stores/useToastStore";

const TOTAL_STEPS = 4;

export default function SetupPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const step = useSetupStore((s) => s.step);
  const setStep = useSetupStore((s) => s.setStep);
  const topic = useSetupStore((s) => s.topic);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const addGladiateur = useSetupStore((s) => s.addGladiateur);
  const buildConfig = useSetupStore((s) => s.buildConfig);
  const settings = useSettingsStore((s) => s.settings);
  const profiles = useSettingsStore((s) => s.profiles);
  const handleEvent = useArenaStore((s) => s.handleEvent);
  const arenaReset = useArenaStore((s) => s.reset);
  const startingRef = useRef(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const canNext = () => {
    switch (step) {
      case 0:
        return topic.trim().length > 0;
      case 1:
        return arbitre.name.trim().length > 0;
      case 2:
        return gladiateurs.length >= 1;
      case 3:
        return true;
      default:
        return false;
    }
  };

  const handleStart = async () => {
    if (startingRef.current) return;
    if (!settings.username.trim()) {
      setError(t("settings.usernameRequired"));
      return;
    }
    startingRef.current = true;
    setStarting(true);
    setError(null);
    arenaReset();
    try {
      const config = buildConfig(settings.username.trim());
      await api.startDiscussion(config, (event) => {
        handleEvent(event);
      });
      navigate("/arena");
    } catch (e: unknown) {
      const msg = extractErrorMessage(e);
      setError(msg || t("errors.generic"));
      toast.error(t("setup.startError"), msg);
      setStarting(false);
      startingRef.current = false;
    }
  };

  const addGladiateurFromProfile = (profile: PredefinedProfile) => {
    addGladiateur({
      id: `glad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      name: t(`profiles.${profile.id}.name`, { defaultValue: profile.name }),
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: t(`profiles.${profile.id}.systemPrompt`, { defaultValue: profile.systemPrompt }),
      llmParams: { ...DEFAULT_LLM_PARAMS },
      sourceProfileId: profile.id,
      initialEmotions: profile.initialEmotions,
    });
  };

  const addEmptyGladiateur = () => {
    addGladiateur({
      id: `glad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
      name: "",
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: "",
      llmParams: { ...DEFAULT_LLM_PARAMS },
    });
  };

  return (
    <>
      <TopBar title={t("setup.title")} />
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Stepper indicator */}
        <div className="flex items-center justify-center gap-2 border-b border-border px-4 py-3">
          {Array.from({ length: TOTAL_STEPS }).map((_, i) => (
            <div key={i} className="flex items-center gap-2">
              <div
                className={cn(
                  "flex h-7 w-7 items-center justify-center rounded-full text-xs font-medium transition-colors",
                  i === step
                    ? "bg-primary text-primary-foreground"
                    : i < step
                      ? "bg-primary/20 text-primary"
                      : "bg-muted text-muted-foreground",
                )}
              >
                {i + 1}
              </div>
              {i < TOTAL_STEPS - 1 && (
                <div
                  className={cn(
                    "h-px w-8",
                    i < step ? "bg-primary" : "bg-border",
                  )}
                />
              )}
            </div>
          ))}
          <span className="ml-3 text-sm font-semibold text-foreground">
            {t("setup.step", { current: step + 1, total: TOTAL_STEPS })}
            {" — "}
            {t(`setup.stepTitle_${step}`)}
          </span>
        </div>

        {/* Step content */}
        <div className="flex-1 overflow-y-auto p-6">
          <div className="mx-auto max-w-2xl">
            {step === 0 && <StepTopic />}
            {step === 1 && <StepArbitre />}
            {step === 2 && (
              <StepGladiateurs
                profiles={profiles}
                onAddFromProfile={addGladiateurFromProfile}
                onAddEmpty={addEmptyGladiateur}
              />
            )}
            {step === 3 && <StepSummary />}
          </div>
        </div>

        {/* Navigation buttons */}
        <div className="flex items-center justify-between border-t border-border px-6 py-3">
          <button
            onClick={() => setStep(Math.max(0, step - 1))}
            disabled={step === 0}
            className="flex items-center gap-1.5 rounded-md border border-border px-4 py-2 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-30"
          >
            <ArrowLeft className="h-4 w-4" />
            {t("setup.previous")}
          </button>

          {error && (
            <p className="text-sm text-destructive">{error}</p>
          )}

          {step < TOTAL_STEPS - 1 ? (
            <button
              onClick={() => setStep(step + 1)}
              disabled={!canNext()}
              className="flex items-center gap-1.5 rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-30"
            >
              {t("setup.next")}
              <ArrowRight className="h-4 w-4" />
            </button>
          ) : (
            <button
              onClick={handleStart}
              disabled={starting || !canNext()}
              className="flex items-center gap-1.5 rounded-md bg-primary px-5 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-30"
            >
              <Play className="h-4 w-4" />
              {t("setup.start")}
            </button>
          )}
        </div>
      </div>
    </>
  );
}

const DISCUSSION_MODES: DiscussionMode[] = [
  "debate", "ideation", "coConstruction", "userDriven",
  "socratic", "tutorial", "critiqueReview", "collaborativeFiction",
];

const DOCUMENT_FORMATS: DocumentFormat[] = ["none", "txt", "md", "csv"];

const RAG_SUPPORTED_EXTENSIONS = new Set([
  "txt", "pdf", "docx", "pptx",
  "py", "rs", "ts", "tsx", "js", "jsx", "java", "c", "cpp", "go", "rb", "php",
  "swift", "kt", "cs", "yaml", "yml", "json", "xml", "html", "css", "sql",
  "md", "csv", "toml", "sh", "log",
]);

function StepTopic() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const setTopic = useSetupStore((s) => s.setTopic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const setDiscussionLanguage = useSetupStore((s) => s.setDiscussionLanguage);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const setDiscussionMode = useSetupStore((s) => s.setDiscussionMode);
  const documentFormat = useSetupStore((s) => s.documentFormat);
  const setDocumentFormat = useSetupStore((s) => s.setDocumentFormat);
  const argumentMapEnabled = useSetupStore((s) => s.argumentMapEnabled);
  const setArgumentMapEnabled = useSetupStore((s) => s.setArgumentMapEnabled);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const setMaxTurns = useSetupStore((s) => s.setMaxTurns);
  const userInterventionTimeoutSecs = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const setUserTimeout = useSetupStore((s) => s.setUserTimeout);
  const ragDocuments = useSetupStore((s) => s.ragDocuments);
  const addRagDocument = useSetupStore((s) => s.addRagDocument);
  const removeRagDocument = useSetupStore((s) => s.removeRagDocument);
  const ollamaModel = useSettingsStore((s) => s.settings.ollamaModel);
  const embeddingModel = useSettingsStore((s) => s.settings.embeddingModel);
  const [ragImporting, setRagImporting] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  const importFiles = async (paths: string[]) => {
    setRagImporting(true);
    try {
      for (const filePath of paths) {
        try {
          const doc = await api.importRagDocument(filePath);
          addRagDocument(doc);
        } catch (e: unknown) {
          const detail = extractErrorMessage(e);
          const fileName = filePath.split(/[\\/]/).pop() ?? filePath;
          toast.error(t("setup.ragImportError", { file: fileName }), detail);
        }
      }
    } finally {
      setRagImporting(false);
    }
  };

  const handleRagImport = async () => {
    const effectiveModel = embeddingModel || ollamaModel;
    if (!effectiveModel) {
      toast.warning(t("setup.ragNoModel"));
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: true,
        filters: [{
          name: "Documents",
          extensions: [...RAG_SUPPORTED_EXTENSIONS],
        }],
      });
      if (!selected || selected.length === 0) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      await importFiles(paths);
    } catch (e: unknown) {
      toast.error(t("setup.ragImportError", { file: "" }), extractErrorMessage(e));
    }
  };

  // Ref to always call the latest drop handler without re-subscribing
  const dropHandlerRef = useRef<(paths: string[]) => void>(() => {});
  dropHandlerRef.current = (paths: string[]) => {
    const effectiveModel = embeddingModel || ollamaModel;
    if (!effectiveModel) {
      toast.warning(t("setup.ragNoModel"));
      return;
    }
    const validPaths = paths.filter((p) => {
      const ext = p.split(".").pop()?.toLowerCase() ?? "";
      return RAG_SUPPORTED_EXTENSIONS.has(ext);
    });
    if (validPaths.length === 0) {
      toast.warning(t("setup.ragUnsupportedFormat"));
      return;
    }
    importFiles(validPaths);
  };

  // Subscribe to Tauri drag-drop events (global webview-level)
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let mounted = true;
    (async () => {
      const { getCurrentWebviewWindow } = await import("@tauri-apps/api/webviewWindow");
      const unsub = await getCurrentWebviewWindow().onDragDropEvent((event) => {
        if (!mounted) return;
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setDragOver(true);
        } else if (event.payload.type === "leave") {
          setDragOver(false);
        } else if (event.payload.type === "drop") {
          setDragOver(false);
          dropHandlerRef.current(event.payload.paths);
        }
      });
      if (mounted) {
        unlisten = unsub;
      } else {
        unsub();
      }
    })();
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Globe className="h-4 w-4 text-primary" />
          {t("setup.discussionLanguage")}
        </label>
        <div className="flex gap-2">
          {(["fr", "en", "zh"] as const).map((lang) => (
            <button
              key={lang}
              onClick={() => setDiscussionLanguage(lang)}
              className={cn(
                "flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-sm transition-colors",
                discussionLanguage === lang
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              <Globe className="h-3.5 w-3.5" />
              {t(`languages.${lang}`)}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-1.5">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Repeat className="h-4 w-4 text-primary" />
          {t("setup.maxTurns")}
        </label>
        <input
          type="number"
          min={1}
          max={100}
          value={maxTurns ?? ""}
          onChange={(e) =>
            setMaxTurns(e.target.value ? parseInt(e.target.value) : null)
          }
          placeholder={t("setup.maxTurnsPlaceholder")}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      {/* Discussion mode selector */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <MessageSquare className="h-4 w-4 text-primary" />
          {t("setup.discussionMode")}
        </label>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
          {DISCUSSION_MODES.map((mode) => (
            <button
              key={mode}
              onClick={() => setDiscussionMode(mode)}
              className={cn(
                "rounded-md border px-3 py-2 text-left transition-colors",
                discussionMode === mode
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              <div className="text-sm font-medium">{t(`setup.mode_${mode}`)}</div>
              <div className="mt-0.5 text-xs opacity-70">{t(`setup.mode_${mode}Desc`)}</div>
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <MessageSquare className="h-4 w-4 text-primary" />
          {t("setup.topic")}
        </label>
        <textarea
          value={topic}
          onChange={(e) => setTopic(e.target.value)}
          placeholder={t("setup.topicPlaceholder")}
          rows={4}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      {/* Argument Map toggle */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Network className="h-4 w-4 text-primary" />
          {t("setup.argumentMap")}
        </label>
        <p className="text-xs text-muted-foreground">{t("setup.argumentMapDesc")}</p>
        <div className="flex items-center gap-3">
          <button
            type="button"
            role="switch"
            aria-checked={argumentMapEnabled}
            onClick={() => setArgumentMapEnabled(!argumentMapEnabled)}
            className={cn(
              "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
              argumentMapEnabled ? "bg-primary" : "bg-muted",
            )}
          >
            <span
              className={cn(
                "pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform",
                argumentMapEnabled ? "translate-x-4" : "translate-x-0",
              )}
            />
          </button>
          <span className="text-sm text-muted-foreground">
            {argumentMapEnabled ? t("setup.switchYes") : t("setup.switchNo")}
          </span>
        </div>
      </div>

      {/* RAG Knowledge Base */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Database className="h-4 w-4 text-purple-500" />
          {t("setup.ragKnowledgeBase")}
        </label>
        <p className="text-xs text-muted-foreground">{t("setup.ragDesc")}</p>

        {!embeddingModel && ollamaModel && (
          <div className="flex items-start gap-2 rounded-md border border-purple-500/20 bg-purple-500/5 px-3 py-2 text-xs text-muted-foreground">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-purple-500" />
            <span>{t("setup.ragRecommendModel")}</span>
          </div>
        )}

        {dragOver && (
          <div className="flex flex-col items-center justify-center rounded-md border-2 border-dashed border-purple-500/50 bg-purple-500/5 py-6">
            <Upload className="h-8 w-8 text-purple-500" />
            <p className="mt-2 text-sm font-medium text-purple-500">{t("setup.ragDropFiles")}</p>
          </div>
        )}

        <div className="flex items-center gap-2">
          <button
            onClick={handleRagImport}
            disabled={ragImporting}
            className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-accent disabled:opacity-50"
          >
            {ragImporting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Plus className="h-3.5 w-3.5" />
            )}
            {ragImporting ? t("setup.ragImporting") : t("setup.ragImportFiles")}
          </button>
          <span className="text-xs text-muted-foreground">{t("setup.ragDropHint")}</span>
        </div>

        {ragDocuments.length > 0 && (
          <div className="space-y-1.5">
            {ragDocuments.map((doc) => (
              <div
                key={doc.docId}
                className="flex items-center justify-between rounded-md border border-border bg-card px-3 py-2"
              >
                <div className="flex items-center gap-2 text-sm">
                  <Database className="h-3.5 w-3.5 text-purple-500" />
                  <span className="font-medium text-foreground">{doc.fileName}</span>
                  <span className="text-xs text-muted-foreground">
                    .{doc.format} — {doc.chunkCount} {t("setup.ragChunks")}, {doc.charCount.toLocaleString()} {t("setup.ragChars")}
                  </span>
                </div>
                <button
                  onClick={async () => {
                    try {
                      await api.removeRagDocument(doc.docId);
                      removeRagDocument(doc.docId);
                    } catch (e: unknown) {
                      toast.error(t("setup.ragRemoveError"), extractErrorMessage(e));
                    }
                  }}
                  className="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Document format selector */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <FileText className="h-4 w-4 text-primary" />
          {t("setup.documentFormat")}
        </label>
        <p className="text-xs text-muted-foreground">{t("setup.documentFormatDesc")}</p>
        <div className="grid grid-cols-4 gap-2">
          {DOCUMENT_FORMATS.map((fmt) => (
            <button
              key={fmt}
              onClick={() => setDocumentFormat(fmt)}
              className={cn(
                "rounded-md border px-3 py-2 text-center text-sm transition-colors",
                documentFormat === fmt
                  ? "border-primary bg-primary/10 text-primary font-medium"
                  : "border-border text-muted-foreground hover:bg-accent",
              )}
            >
              {t(`setup.docFormat_${fmt}`)}
            </button>
          ))}
        </div>
      </div>

      <div className="space-y-1.5">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Clock className="h-4 w-4 text-primary" />
          {t("setup.userTimeout")}
        </label>
        <input
          type="number"
          min={30}
          max={600}
          value={userInterventionTimeoutSecs}
          onChange={(e) => setUserTimeout(parseInt(e.target.value) || 120)}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>
    </div>
  );
}

function StepArbitre() {
  const { t } = useTranslation();
  const arbitre = useSetupStore((s) => s.arbitre);
  const updateArbitre = useSetupStore((s) => s.updateArbitre);
  const updateArbitreLlm = useSetupStore((s) => s.updateArbitreLlm);
  const arbitreProfiles = useSettingsStore((s) => s.arbitreProfiles);
  const saveArbitreProfile = useSettingsStore((s) => s.saveArbitreProfile);
  const deleteArbitreProfile = useSettingsStore((s) => s.deleteArbitreProfile);
  const hasTavilyKey = !!useSettingsStore((s) => s.settings.tavilyApiKey);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const [showLlm, setShowLlm] = useState(false);
  const [showSaveForm, setShowSaveForm] = useState(false);
  const [savePersonality, setSavePersonality] = useState("");

  const handleProfileChange = (profileId: string) => {
    if (profileId === "") return;
    const profile = arbitreProfiles.find((p) => p.id === profileId);
    if (profile) {
      updateArbitre({
        name: t(`profiles.${profile.id}.name`, { defaultValue: profile.name }),
        systemPrompt: t(`profiles.${profile.id}.systemPrompt`, { defaultValue: profile.systemPrompt }),
      });
    }
  };

  // Find which profile matches the current config (if any)
  const currentProfile = arbitreProfiles.find(
    (p) =>
      t(`profiles.${p.id}.name`, { defaultValue: p.name }) === arbitre.name &&
      t(`profiles.${p.id}.systemPrompt`, { defaultValue: p.systemPrompt }) === arbitre.systemPrompt,
  );
  const currentProfileId = currentProfile?.id ?? "";
  const isCustomConfig = !currentProfileId;
  const isCustomProfile = currentProfile && !currentProfile.isBuiltin;

  const handleSaveAsProfile = async () => {
    const id = `arb-custom-${Date.now()}`;
    await saveArbitreProfile({
      id,
      name: arbitre.name,
      personality: savePersonality || arbitre.name,
      systemPrompt: arbitre.systemPrompt,
      isBuiltin: false,
      profileType: "arbitre",
      category: "arbitre",
    });
    setShowSaveForm(false);
    setSavePersonality("");
  };

  const handleDeleteProfile = async () => {
    if (!currentProfile || currentProfile.isBuiltin) return;
    await deleteArbitreProfile(currentProfile.id);
    updateArbitre({ name: "", systemPrompt: "" });
  };

  return (
    <div className="space-y-4">
      {/* Optional web search for introduction */}
      {hasTavilyKey && (
        <div className="space-y-2">
          <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
            <Globe className="h-4 w-4 text-primary" />
            {t("setup.arbitreWebSearchTitle")}
          </label>
          <div className="flex items-center gap-3">
            <button
              type="button"
              role="switch"
              aria-checked={arbitre.webSearchIntro ?? false}
              onClick={() => updateArbitre({ webSearchIntro: !(arbitre.webSearchIntro ?? false) })}
              className={cn(
                "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
                (arbitre.webSearchIntro ?? false) ? "bg-primary" : "bg-muted",
              )}
            >
              <span
                className={cn(
                  "pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform",
                  (arbitre.webSearchIntro ?? false) ? "translate-x-4" : "translate-x-0",
                )}
              />
            </button>
            <span className="text-sm text-muted-foreground">
              {(arbitre.webSearchIntro ?? false) ? t("setup.switchYes") : t("setup.switchNo")}
            </span>
          </div>
        </div>
      )}

      {/* Optional Wikipedia search for introduction (always available — free) */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <BookOpen className="h-4 w-4 text-green-600" />
          {t("setup.arbitreWikiSearchTitle")}
        </label>
        <div className="flex items-center gap-3">
          <button
            type="button"
            role="switch"
            aria-checked={arbitre.wikiSearchIntro ?? false}
            onClick={() => updateArbitre({ wikiSearchIntro: !(arbitre.wikiSearchIntro ?? false) })}
            className={cn(
              "relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
              (arbitre.wikiSearchIntro ?? false) ? "bg-green-600" : "bg-muted",
            )}
          >
            <span
              className={cn(
                "pointer-events-none inline-block h-4 w-4 rounded-full bg-background shadow-sm transition-transform",
                (arbitre.wikiSearchIntro ?? false) ? "translate-x-4" : "translate-x-0",
              )}
            />
          </button>
          <span className="text-sm text-muted-foreground">
            {(arbitre.wikiSearchIntro ?? false) ? t("setup.switchYes") : t("setup.switchNo")}
          </span>
        </div>
      </div>

      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Shuffle className="h-4 w-4 text-primary" />
          {t("setup.turnDistribution")}
        </label>
        {discussionMode === "userDriven" || discussionMode === "collaborativeFiction" ? (
          <p className="rounded-md border border-border bg-muted/50 px-3 py-2 text-sm text-muted-foreground">
            {discussionMode === "userDriven"
              ? t("setup.userDrivenNoTurnDist")
              : t("setup.fictionNoTurnDist")}
          </p>
        ) : (
          <div className="grid grid-cols-2 gap-2">
            {(
              ["sequential", "random", "democratic", "authoritarian"] as const
            ).map((dist) => (
              <button
                key={dist}
                onClick={() => updateArbitre({ turnDistribution: dist })}
                className={cn(
                  "rounded-md border px-3 py-2 text-left transition-colors",
                  arbitre.turnDistribution === dist
                    ? "border-primary bg-primary/10 text-primary"
                    : "border-border text-muted-foreground hover:bg-accent",
                )}
              >
                <div className="text-sm font-medium">{t(`setup.${dist}`)}</div>
                <div className="mt-0.5 text-xs opacity-70">
                  {t(`setup.${dist}Desc`)}
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Profile selector */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <UserCircle className="h-4 w-4 text-primary" />
          {t("setup.arbitreProfile")}
        </label>
        <div className="flex gap-2">
          <select
            value={currentProfileId}
            onChange={(e) => handleProfileChange(e.target.value)}
            className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          >
            {isCustomConfig && (
              <option value="">{t("setup.arbitreCustom")}</option>
            )}
            {arbitreProfiles.map((p) => (
              <option key={p.id} value={p.id}>
                {getProfileEmoji(p.name, p.systemPrompt)} {t(`profiles.${p.id}.name`, { defaultValue: p.name })} — {t(`profiles.${p.id}.personality`, { defaultValue: p.personality })}
              </option>
            ))}
          </select>
          {isCustomProfile && (
            <button
              onClick={handleDeleteProfile}
              title={t("setup.deleteArbitreProfile")}
              className="rounded-md border border-border px-2 py-2 text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
            >
              <Trash2 className="h-4 w-4" />
            </button>
          )}
        </div>
      </div>

      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Tag className="h-4 w-4 text-primary" />
          {t("setup.arbitreName")}
        </label>
        <input
          type="text"
          value={arbitre.name}
          onChange={(e) => updateArbitre({ name: e.target.value })}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
      </div>

      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <FileText className="h-4 w-4 text-primary" />
          {t("setup.arbitrePrompt")}
        </label>
        <PersonaEditor
          systemPrompt={arbitre.systemPrompt}
          profileType="arbitre"
          discussionLanguage={discussionLanguage}
          onChange={(prompt) => updateArbitre({ systemPrompt: prompt })}
        />
      </div>

      {/* Save as custom profile */}
      {isCustomConfig && arbitre.name.trim() && arbitre.systemPrompt.trim() && (
        <>
          {!showSaveForm ? (
            <button
              onClick={() => setShowSaveForm(true)}
              className="flex items-center gap-1.5 text-sm text-primary hover:text-primary/80"
            >
              <Save className="h-4 w-4" />
              {t("setup.saveArbitreProfile")}
            </button>
          ) : (
            <div className="flex items-end gap-2 rounded-md border border-border bg-card p-3">
              <div className="flex-1 space-y-1">
                <label className="text-xs text-muted-foreground">
                  {t("setup.arbitreProfilePersonality")}
                </label>
                <input
                  type="text"
                  value={savePersonality}
                  onChange={(e) => setSavePersonality(e.target.value)}
                  placeholder={t("setup.personalityPlaceholder")}
                  className="w-full rounded-md border border-input bg-background px-3 py-1.5 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                />
              </div>
              <button
                onClick={handleSaveAsProfile}
                className="rounded-md bg-primary px-3 py-1.5 text-sm text-primary-foreground hover:bg-primary/90"
              >
                {t("settings.save")}
              </button>
              <button
                onClick={() => { setShowSaveForm(false); setSavePersonality(""); }}
                className="rounded-md border border-border px-3 py-1.5 text-sm text-muted-foreground hover:bg-accent"
              >
                ✕
              </button>
            </div>
          )}
        </>
      )}

      <button
        onClick={() => setShowLlm(!showLlm)}
        className="flex w-full items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground hover:text-foreground/80"
      >
        <Sliders className="h-4 w-4 text-primary" />
        {t("setup.llmParams")}
        <span className="ml-auto">
          {showLlm ? (
            <ChevronUp className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          )}
        </span>
      </button>
      {showLlm && (
        <LlmParamsForm
          params={arbitre.llmParams}
          onChange={updateArbitreLlm}
        />
      )}
    </div>
  );
}

const CATEGORY_ORDER = ["personnel", "experts", "imaginaires", "personnalites", "metiers", "autres"] as const;

function StepGladiateurs({
  profiles,
  onAddFromProfile,
  onAddEmpty,
}: {
  profiles: PredefinedProfile[];
  onAddFromProfile: (p: PredefinedProfile) => void;
  onAddEmpty: () => void;
}) {
  const { t } = useTranslation();
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const removeGladiateur = useSetupStore((s) => s.removeGladiateur);
  const updateGladiateur = useSetupStore((s) => s.updateGladiateur);
  const updateGladiateurLlm = useSetupStore((s) => s.updateGladiateurLlm);
  const reorderGladiateurs = useSetupStore((s) => s.reorderGladiateurs);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const webSearchPool = useSetupStore((s) => s.webSearchPool);
  const setWebSearchPool = useSetupStore((s) => s.setWebSearchPool);
  const wikiSearchPool = useSetupStore((s) => s.wikiSearchPool);
  const setWikiSearchPool = useSetupStore((s) => s.setWikiSearchPool);
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const saveProfile = useSettingsStore((s) => s.saveProfile);
  const deleteProfile = useSettingsStore((s) => s.deleteProfile);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [collapsedCats, setCollapsedCats] = useState<Set<string>>(
    () => new Set(CATEGORY_ORDER),
  );
  const [savedIds, setSavedIds] = useState<Set<string>>(new Set());
  const dragFrom = useRef<number | null>(null);
  const dropTo = useRef<number | null>(null);
  const [dragOverIdx, setDragOverIdx] = useState<number | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const cardsRef = useRef<HTMLDivElement>(null);

  // Pointer-event-based drag-and-drop (HTML5 drag doesn't work in Tauri WebView)
  useEffect(() => {
    if (!isDragging) return;
    document.body.style.userSelect = "none";

    const onMove = (e: PointerEvent) => {
      if (dragFrom.current === null || !cardsRef.current) return;
      const y = e.clientY;
      const cards = Array.from(cardsRef.current.children) as HTMLElement[];
      let target: number | null = null;
      for (let i = 0; i < cards.length; i++) {
        const rect = cards[i].getBoundingClientRect();
        if (y >= rect.top && y <= rect.bottom) {
          target = i;
          break;
        }
      }
      const idx = target !== null && target !== dragFrom.current ? target : null;
      dropTo.current = idx;
      setDragOverIdx(idx);
    };

    const onUp = () => {
      if (dragFrom.current !== null && dropTo.current !== null) {
        reorderGladiateurs(dragFrom.current, dropTo.current);
      }
      dragFrom.current = null;
      dropTo.current = null;
      setDragOverIdx(null);
      setIsDragging(false);
    };

    document.addEventListener("pointermove", onMove);
    document.addEventListener("pointerup", onUp);
    return () => {
      document.removeEventListener("pointermove", onMove);
      document.removeEventListener("pointerup", onUp);
      document.body.style.userSelect = "";
    };
  }, [isDragging, reorderGladiateurs]);

  // Group profiles by category
  const grouped = CATEGORY_ORDER.map((cat) => ({
    category: cat,
    profiles: profiles.filter((p) => p.category === cat),
  })).filter((g) => g.profiles.length > 0);

  const handleToggleEmotionDriven = () => {
    updateSettings({ emotionDriven: !settings.emotionDriven });
    saveSettings();
  };

  const getSourceProfile = (g: GladIAteurConfig): PredefinedProfile | undefined =>
    g.sourceProfileId ? profiles.find((p) => p.id === g.sourceProfileId) : undefined;

  const handleSaveGladiateur = async (g: GladIAteurConfig) => {
    const source = getSourceProfile(g);
    const id = source && !source.isBuiltin ? source.id : `glad-custom-${Date.now()}`;
    await saveProfile({
      id,
      name: g.name,
      personality: g.name,
      systemPrompt: g.systemPrompt,
      isBuiltin: false,
      profileType: "gladiateur",
      category: "personnel",
    });
    setSavedIds((prev) => new Set(prev).add(g.id));
    setTimeout(() => setSavedIds((prev) => {
      const next = new Set(prev);
      next.delete(g.id);
      return next;
    }), 2000);
  };

  const isModified = (g: GladIAteurConfig): boolean => {
    const source = getSourceProfile(g);
    if (!source) return false;
    const origName = t(`profiles.${source.id}.name`, { defaultValue: source.name });
    const origPrompt = t(`profiles.${source.id}.systemPrompt`, { defaultValue: source.systemPrompt });
    return g.name !== origName || g.systemPrompt !== origPrompt;
  };

  const handleResetGladiateur = (g: GladIAteurConfig) => {
    const source = getSourceProfile(g);
    if (!source) return;
    updateGladiateur(g.id, {
      name: t(`profiles.${source.id}.name`, { defaultValue: source.name }),
      systemPrompt: t(`profiles.${source.id}.systemPrompt`, { defaultValue: source.systemPrompt }),
    });
  };

  const hasTavilyKey = !!settings.tavilyApiKey.trim();
  const maxSearchBound = (maxTurns ?? 100) * Math.max(gladiateurs.length, 1);

  return (
    <div className="space-y-6">
      {/* Global web search config */}
      {hasTavilyKey && (
        <div className="space-y-2">
          <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
            <Globe className="h-4 w-4 text-primary" />
            {t("setup.webSearchPool")}
          </label>
          <div className="flex items-center gap-3">
            <input
              type="number"
              min={0}
              max={maxSearchBound}
              value={webSearchPool}
              onChange={(e) =>
                setWebSearchPool(
                  Math.max(0, Math.min(maxSearchBound, parseInt(e.target.value) || 0)),
                )
              }
              className="w-24 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <span className="text-sm text-muted-foreground">
              {t("setup.webSearchPoolDesc", { max: maxSearchBound })}
            </span>
          </div>
          {webSearchPool > 0 && (
            <p className="text-xs text-muted-foreground">
              {t("setup.webSearchBudget", {
                count: webSearchPool,
              })}
            </p>
          )}
        </div>
      )}
      {!hasTavilyKey && (
        <div className="flex items-center gap-2 rounded-md border border-dashed border-border px-3 py-2 text-xs text-muted-foreground">
          <Globe className="h-3.5 w-3.5" />
          {t("setup.webSearchNoKey")}
        </div>
      )}

      {/* Global wiki search config */}
      <div className="space-y-2">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <BookOpen className="h-4 w-4 text-green-600" />
          {t("setup.wikiSearchPool")}
        </label>
        <div className="flex items-center gap-3">
          <input
            type="number"
            min={0}
            max={maxSearchBound}
            value={wikiSearchPool}
            onChange={(e) =>
              setWikiSearchPool(
                Math.max(0, Math.min(maxSearchBound, parseInt(e.target.value) || 0)),
              )
            }
            className="w-24 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          />
          <span className="text-sm text-muted-foreground">
            {t("setup.wikiSearchPoolDesc", { max: maxSearchBound })}
          </span>
        </div>
        {wikiSearchPool > 0 && (
          <p className="text-xs text-muted-foreground">
            {t("setup.wikiSearchBudget", {
              count: wikiSearchPool,
            })}
          </p>
        )}
      </div>

      {/* First-turn search rule explanation */}
      {(webSearchPool > 0 || wikiSearchPool > 0) && (
        <div className="flex items-start gap-2 rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-xs text-muted-foreground">
          <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-primary" />
          <span>{t("setup.firstTurnSearchRule")}</span>
        </div>
      )}

      {/* Emotion-driven behavior toggle */}
      <div className="space-y-3">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Heart className="h-4 w-4 text-primary" />
          {t("settings.emotionDriven")}
        </label>
        <div className="flex items-center gap-3">
          <button
            onClick={handleToggleEmotionDriven}
            className={cn(
              "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
              settings.emotionDriven ? "bg-primary" : "bg-muted",
            )}
          >
            <span
              className={cn(
                "pointer-events-none inline-block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform",
                settings.emotionDriven ? "translate-x-5" : "translate-x-0",
              )}
            />
          </button>
          <span className="text-sm text-muted-foreground">
            {t("settings.emotionDrivenDesc")}
          </span>
        </div>
      </div>

      {/* Profile picker grouped by category */}
      <div className="space-y-3">
        <label className="flex items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground">
          <Users className="h-4 w-4 text-primary" />
          {t("setup.selectProfile")}
        </label>
        {grouped.map((group) => {
          const collapsed = collapsedCats.has(group.category);
          return (
            <div key={group.category}>
              <button
                onClick={() =>
                  setCollapsedCats((prev) => {
                    const next = new Set(prev);
                    if (next.has(group.category)) next.delete(group.category);
                    else next.add(group.category);
                    return next;
                  })
                }
                className="mb-1.5 flex w-full items-center gap-1 text-xs font-medium uppercase tracking-wide text-muted-foreground hover:text-foreground"
              >
                {collapsed ? (
                  <ChevronRight className="h-3.5 w-3.5" />
                ) : (
                  <ChevronDown className="h-3.5 w-3.5" />
                )}
                {t(`setup.category_${group.category}`)}
                <span className="font-normal normal-case">({group.profiles.length})</span>
              </button>
              {!collapsed && (
                <div className="flex flex-wrap gap-2">
                  {group.profiles.map((p) => (
                    <div key={p.id} className="group/profile relative">
                      <button
                        onClick={() => onAddFromProfile(p)}
                        className="rounded-md border border-border px-3 py-1.5 text-sm text-foreground transition-colors hover:bg-accent"
                      >
                        <span className="mr-1.5 inline-block">{getProfileEmoji(p.name, p.systemPrompt)}</span>
                        {t(`profiles.${p.id}.name`, { defaultValue: p.name })}
                      </button>
                      {!p.isBuiltin && (
                        <button
                          onClick={(e) => { e.stopPropagation(); deleteProfile(p.id); }}
                          title={t("setup.deleteProfile")}
                          className="absolute -right-1.5 -top-1.5 hidden rounded-full bg-destructive p-0.5 text-destructive-foreground shadow-sm group-hover/profile:block"
                        >
                          <Trash2 className="h-3 w-3" />
                        </button>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
        <div>
          <button
            onClick={onAddEmpty}
            className="rounded-md border border-dashed border-border px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <Plus className="mr-1.5 inline h-3.5 w-3.5" />
            {t("setup.addGladiateur")}
          </button>
        </div>
      </div>

      {gladiateurs.length === 0 && (
        <p className="py-8 text-center text-sm text-muted-foreground">
          {t("setup.minGladiateurs")}
        </p>
      )}

      {/* Gladiateur cards */}
      <div className="space-y-3" ref={cardsRef}>
        {gladiateurs.map((g, idx) => (
          <div
            key={g.id}
            className={cn(
              "rounded-lg border bg-card p-4 transition-colors",
              dragOverIdx === idx ? "border-primary bg-primary/5" : "border-border",
            )}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <GripVertical
                  className="h-4 w-4 shrink-0 cursor-grab text-muted-foreground active:cursor-grabbing"
                  onPointerDown={(e) => {
                    e.preventDefault();
                    dragFrom.current = idx;
                    setIsDragging(true);
                  }}
                />
                <span className="flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
                  {idx + 1}
                </span>
                <EmojiPicker
                  value={g.emoji}
                  autoEmoji={getProfileEmoji(g.name, g.systemPrompt)}
                  onSelect={(emoji) => updateGladiateur(g.id, { emoji })}
                />
                <input
                  type="text"
                  value={g.name}
                  onChange={(e) =>
                    updateGladiateur(g.id, { name: e.target.value })
                  }
                  placeholder={t("setup.gladiateurName")}
                  className="border-none bg-transparent text-sm font-medium text-foreground placeholder:text-muted-foreground focus:outline-none"
                />
              </div>
              <div className="flex items-center gap-1">
                {isModified(g) && (
                  <button
                    onClick={() => handleResetGladiateur(g)}
                    title={t("setup.resetGladiateur")}
                    className="rounded p-1 text-amber-500 hover:bg-amber-500/10"
                  >
                    <RotateCcw className="h-4 w-4" />
                  </button>
                )}
                <button
                  onClick={() =>
                    setExpandedId(expandedId === g.id ? null : g.id)
                  }
                  className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                >
                  {expandedId === g.id ? (
                    <ChevronUp className="h-4 w-4" />
                  ) : (
                    <ChevronDown className="h-4 w-4" />
                  )}
                </button>
                {g.name.trim() && g.systemPrompt.trim() && (
                  <button
                    onClick={() => handleSaveGladiateur(g)}
                    title={t("setup.saveGladiateur")}
                    className={cn(
                      "rounded p-1 transition-colors",
                      savedIds.has(g.id)
                        ? "text-green-500"
                        : "text-muted-foreground hover:bg-accent hover:text-foreground",
                    )}
                  >
                    {savedIds.has(g.id) ? (
                      <Check className="h-4 w-4" />
                    ) : (
                      <Save className="h-4 w-4" />
                    )}
                  </button>
                )}
                <button
                  onClick={() => removeGladiateur(g.id)}
                  className="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            </div>

            {expandedId === g.id && (
              <div className="mt-3 space-y-3 border-t border-border pt-3">
                <div className="space-y-1.5">
                  <label className="text-xs text-muted-foreground">
                    {t("setup.gladiateurPrompt")}
                  </label>
                  <PersonaEditor
                    systemPrompt={g.systemPrompt}
                    profileType="gladiateur"
                    discussionLanguage={discussionLanguage}
                    onChange={(prompt) =>
                      updateGladiateur(g.id, { systemPrompt: prompt })
                    }
                  />
                </div>
                <div>
                  <label className="mb-2 block text-xs text-muted-foreground">
                    {t("setup.llmParams")}
                  </label>
                  <LlmParamsForm
                    params={g.llmParams}
                    onChange={(patch) => updateGladiateurLlm(g.id, patch)}
                  />
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function StepSummary() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const arbitre = useSetupStore((s) => s.arbitre);
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const discussionMode = useSetupStore((s) => s.discussionMode);
  const documentFormat = useSetupStore((s) => s.documentFormat);
  const webSearchPool = useSetupStore((s) => s.webSearchPool);
  const wikiSearchPool = useSetupStore((s) => s.wikiSearchPool);
  const argumentMapEnabled = useSetupStore((s) => s.argumentMapEnabled);
  const ragDocuments = useSetupStore((s) => s.ragDocuments);
  const emotionDriven = useSettingsStore((s) => s.settings.emotionDriven);

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-border bg-card p-4 space-y-4">
        {/* Discussion settings */}
        <SummaryRow label={t("setup.summaryDiscussionMode")} value={t(`setup.mode_${discussionMode}`)} />
        <SummaryRow label={t("setup.summaryTopic")} value={topic} />
        <SummaryRow label={t("setup.summaryLanguage")} value={t(`languages.${discussionLanguage}`)} />
        <SummaryRow
          label={t("setup.summaryTurns")}
          value={maxTurns != null ? String(maxTurns) : t("setup.maxTurnsPlaceholder")}
        />

        <hr className="border-border" />

        {/* IArbitre settings */}
        <SummaryRow label={t("setup.summaryArbitre")} value={arbitre.name} />
        <SummaryRow
          label={t("setup.summaryTurnDist")}
          value={discussionMode === "userDriven"
            ? t("setup.userDrivenNoTurnDist")
            : discussionMode === "collaborativeFiction"
              ? t("setup.fictionNoTurnDist")
              : t(`setup.${arbitre.turnDistribution}`)}
        />
        {documentFormat !== "none" && (
          <SummaryRow label={t("setup.summaryDocFormat")} value={`.${documentFormat}`} />
        )}
        {argumentMapEnabled && (
          <SummaryRow label={t("setup.summaryArgumentMap")} value={t("setup.switchYes")} />
        )}
        {(arbitre.webSearchIntro ?? false) && (
          <SummaryRow label={t("setup.summaryWebIntro")} value="1" />
        )}
        {(arbitre.wikiSearchIntro ?? false) && (
          <SummaryRow label={t("setup.summaryWikiIntro")} value="1" />
        )}

        <hr className="border-border" />

        {/* GladIAteurs */}
        <div>
          <p className="text-xs text-muted-foreground">{t("setup.summaryGladiateurs")}</p>
          {gladiateurs.length === 0 ? (
            <p className="mt-1 text-sm font-medium text-foreground">-</p>
          ) : (
            <div className="mt-1 space-y-1">
              {gladiateurs.map((g) => (
                <div key={g.id} className="flex items-center gap-2 text-sm text-foreground">
                  <span>{g.emoji ?? getProfileEmoji(g.name, g.systemPrompt)}</span>
                  <span className="font-medium">{g.name}</span>
                </div>
              ))}
            </div>
          )}
        </div>
        <SummaryRow
          label={t("setup.summaryEmotionDriven")}
          value={emotionDriven ? t("setup.switchYes") : t("setup.switchNo")}
        />
        {webSearchPool > 0 && (
          <SummaryRow
            label={t("setup.summaryWebPool")}
            value={t("setup.webSearchBudget", { count: webSearchPool })}
          />
        )}
        {wikiSearchPool > 0 && (
          <SummaryRow
            label={t("setup.summaryWikiPool")}
            value={t("setup.wikiSearchBudget", { count: wikiSearchPool })}
          />
        )}
        {ragDocuments.length > 0 && (
          <div>
            <p className="text-xs text-muted-foreground">{t("setup.summaryRag")}</p>
            <div className="mt-1 space-y-0.5">
              {ragDocuments.map((doc) => (
                <p key={doc.docId} className="text-sm font-medium text-foreground">
                  {doc.fileName} ({doc.chunkCount} {t("setup.ragChunks")})
                </p>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="mt-0.5 text-sm font-medium text-foreground">{value}</p>
    </div>
  );
}
