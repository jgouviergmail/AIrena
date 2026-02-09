import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Globe,
  Play,
  Plus,
  RotateCcw,
  Save,
  Search,
  Trash2,
} from "lucide-react";
import { TopBar } from "@/components/layout/TopBar";
import { LlmParamsForm } from "@/components/setup/LlmParamsForm";
import { EmojiPicker } from "@/components/setup/EmojiPicker";
import { getProfileEmoji } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { useArenaStore } from "@/stores/useArenaStore";
import { cn } from "@/lib/utils";
import { DEFAULT_LLM_PARAMS } from "@/lib/types";
import type { GladIAteurConfig, PredefinedProfile } from "@/lib/types";
import * as api from "@/lib/tauri-api";

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
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg || t("errors.generic"));
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
          <span className="ml-3 text-xs text-muted-foreground">
            {t("setup.step", { current: step + 1, total: TOTAL_STEPS })}
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

function StepTopic() {
  const { t } = useTranslation();
  const topic = useSetupStore((s) => s.topic);
  const setTopic = useSetupStore((s) => s.setTopic);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const setDiscussionLanguage = useSetupStore((s) => s.setDiscussionLanguage);
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const setMaxTurns = useSetupStore((s) => s.setMaxTurns);
  const userInterventionTimeoutSecs = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const setUserTimeout = useSetupStore((s) => s.setUserTimeout);

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
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

      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
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

      <div className="grid grid-cols-2 gap-4">
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">
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
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">
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
      {/* Profile selector */}
      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
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
        <label className="text-sm font-medium text-foreground">
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
        <label className="text-sm font-medium text-foreground">
          {t("setup.arbitrePrompt")}
        </label>
        <textarea
          value={arbitre.systemPrompt}
          onChange={(e) => updateArbitre({ systemPrompt: e.target.value })}
          rows={4}
          className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
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

      <div className="space-y-2">
        <label className="text-sm font-medium text-foreground">
          {t("setup.turnDistribution")}
        </label>
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
      </div>

      {/* Optional web search for introduction */}
      {hasTavilyKey && (
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
          <div className="flex items-center gap-1.5 text-sm text-muted-foreground">
            <Search className="h-3.5 w-3.5" />
            {t("setup.arbitreWebSearchIntro")}
          </div>
        </div>
      )}

      <button
        onClick={() => setShowLlm(!showLlm)}
        className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
      >
        {showLlm ? (
          <ChevronUp className="h-4 w-4" />
        ) : (
          <ChevronDown className="h-4 w-4" />
        )}
        {t("setup.llmParams")}
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
  const maxTurns = useSetupStore((s) => s.maxTurns);
  const webSearchMaxPerGladiateur = useSetupStore((s) => s.webSearchMaxPerGladiateur);
  const setWebSearchMaxPerGladiateur = useSetupStore((s) => s.setWebSearchMaxPerGladiateur);
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const saveProfile = useSettingsStore((s) => s.saveProfile);
  const deleteProfile = useSettingsStore((s) => s.deleteProfile);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [collapsedCats, setCollapsedCats] = useState<Set<string>>(
    () => new Set(CATEGORY_ORDER),
  );
  const [savedIds, setSavedIds] = useState<Set<string>>(new Set());

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
  const maxSearchBound = maxTurns ?? 100;

  return (
    <div className="space-y-6">
      {/* Emotion-driven behavior toggle */}
      <div className="space-y-3">
        <label className="text-sm font-medium text-foreground">
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

      {/* Global web search config */}
      {hasTavilyKey && (
        <div className="space-y-2">
          <label className="text-sm font-medium text-foreground">
            <Search className="mr-1.5 inline h-3.5 w-3.5" />
            {t("setup.webSearchMaxPerGladiateur")}
          </label>
          <div className="flex items-center gap-3">
            <input
              type="number"
              min={0}
              max={maxSearchBound}
              value={webSearchMaxPerGladiateur}
              onChange={(e) =>
                setWebSearchMaxPerGladiateur(
                  Math.max(0, Math.min(maxSearchBound, parseInt(e.target.value) || 0)),
                )
              }
              className="w-24 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            />
            <span className="text-sm text-muted-foreground">
              {t("setup.webSearchMaxPerGladiateurDesc", { max: maxSearchBound })}
            </span>
          </div>
          {webSearchMaxPerGladiateur > 0 && (
            <p className="text-xs text-muted-foreground">
              {t("setup.webSearchBudget", {
                count: webSearchMaxPerGladiateur * gladiateurs.length,
              })}
            </p>
          )}
        </div>
      )}
      {!hasTavilyKey && (
        <div className="flex items-center gap-2 rounded-md border border-dashed border-border px-3 py-2 text-xs text-muted-foreground">
          <Search className="h-3.5 w-3.5" />
          {t("setup.webSearchNoKey")}
        </div>
      )}

      {/* Profile picker grouped by category */}
      <div className="space-y-3">
        <label className="text-sm font-medium text-foreground">
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
      <div className="space-y-3">
        {gladiateurs.map((g, idx) => (
          <div
            key={g.id}
            className="rounded-lg border border-border bg-card p-4"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
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
                  <textarea
                    value={g.systemPrompt}
                    onChange={(e) =>
                      updateGladiateur(g.id, {
                        systemPrompt: e.target.value,
                      })
                    }
                    rows={3}
                    className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
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
  const userInterventionTimeoutSecs = useSetupStore((s) => s.userInterventionTimeoutSecs);
  const webSearchMaxPerGladiateur = useSetupStore((s) => s.webSearchMaxPerGladiateur);

  return (
    <div className="space-y-6">
      <div className="rounded-lg border border-border bg-card p-4 space-y-3">
        <SummaryRow label={t("setup.topic")} value={topic} />
        <SummaryRow
          label={t("setup.discussionLanguage")}
          value={t(`languages.${discussionLanguage}`)}
        />
        <SummaryRow
          label={t("setup.maxTurns")}
          value={maxTurns != null ? String(maxTurns) : t("setup.maxTurnsPlaceholder")}
        />
        <SummaryRow
          label={t("setup.userTimeout")}
          value={`${userInterventionTimeoutSecs}s`}
        />
        <SummaryRow label={t("setup.arbitreName")} value={arbitre.name} />
        <SummaryRow
          label={t("setup.turnDistribution")}
          value={t(`setup.${arbitre.turnDistribution}`)}
        />
        {(arbitre.webSearchIntro ?? false) && (
          <SummaryRow
            label={t("setup.arbitreWebSearchIntro")}
            value="1"
          />
        )}
        <SummaryRow
          label={t("setup.gladiateurs")}
          value={gladiateurs.map((g) => g.name).join(", ") || "-"}
        />
        {webSearchMaxPerGladiateur > 0 && (
          <SummaryRow
            label={t("setup.webSearch")}
            value={t("setup.webSearchBudget", {
              count: webSearchMaxPerGladiateur * gladiateurs.length + ((arbitre.webSearchIntro ?? false) ? 1 : 0),
            })}
          />
        )}
      </div>
    </div>
  );
}

function SummaryRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between">
      <span className="text-sm text-muted-foreground">{label}</span>
      <span className="text-sm font-medium text-foreground">{value}</span>
    </div>
  );
}
