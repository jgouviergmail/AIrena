import { useState } from "react";
import { useTranslation } from "react-i18next";
import { BookOpen, ChevronDown, ChevronUp, FileText, Globe, Shuffle, Sliders, Save, Tag, Trash2, UserCircle } from "lucide-react";
import { LlmParamsForm } from "@/components/setup/LlmParamsForm";
import { PersonaEditor } from "@/components/setup/PersonaEditor";
import { getProfileEmoji } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { inputClass, OptionCard, SectionLabel, Toggle } from "./shared";

const TURN_DISTRIBUTIONS = ["sequential", "random", "democratic", "authoritarian"] as const;

export function StepArbitre() {
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

  const fixedDistribution = discussionMode === "userDriven" || discussionMode === "collaborativeFiction";

  return (
    <div className="space-y-4">
      {/* Optional web search for introduction */}
      {hasTavilyKey && (
        <div className="space-y-2">
          <SectionLabel icon={Globe}>{t("setup.arbitreWebSearchTitle")}</SectionLabel>
          <Toggle
            checked={arbitre.webSearchIntro ?? false}
            onChange={(next) => updateArbitre({ webSearchIntro: next })}
            caption={(arbitre.webSearchIntro ?? false) ? t("setup.switchYes") : t("setup.switchNo")}
          />
        </div>
      )}

      {/* Optional Wikipedia search for introduction (always available — free) */}
      <div className="space-y-2">
        <SectionLabel icon={BookOpen} iconClassName="text-green-600">{t("setup.arbitreWikiSearchTitle")}</SectionLabel>
        <Toggle
          checked={arbitre.wikiSearchIntro ?? false}
          onChange={(next) => updateArbitre({ wikiSearchIntro: next })}
          caption={(arbitre.wikiSearchIntro ?? false) ? t("setup.switchYes") : t("setup.switchNo")}
          activeClassName="bg-green-600"
        />
      </div>

      <div className="space-y-2">
        <SectionLabel icon={Shuffle}>{t("setup.turnDistribution")}</SectionLabel>
        {fixedDistribution ? (
          <p className="rounded-md border border-border bg-muted/50 px-3 py-2 text-sm text-muted-foreground">
            {discussionMode === "userDriven" ? t("setup.userDrivenNoTurnDist") : t("setup.fictionNoTurnDist")}
          </p>
        ) : (
          <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
            {TURN_DISTRIBUTIONS.map((dist) => (
              <OptionCard
                key={dist}
                selected={arbitre.turnDistribution === dist}
                title={t(`setup.${dist}`)}
                description={t(`setup.${dist}Desc`)}
                onClick={() => updateArbitre({ turnDistribution: dist })}
              />
            ))}
          </div>
        )}
      </div>

      {/* Profile selector */}
      <div className="space-y-2">
        <SectionLabel icon={UserCircle}>{t("setup.arbitreProfile")}</SectionLabel>
        <div className="flex gap-2">
          <select
            value={currentProfileId}
            onChange={(e) => handleProfileChange(e.target.value)}
            className={`${inputClass} flex-1`}
          >
            {isCustomConfig && <option value="">{t("setup.arbitreCustom")}</option>}
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
        <SectionLabel icon={Tag}>{t("setup.arbitreName")}</SectionLabel>
        <input
          type="text"
          value={arbitre.name}
          onChange={(e) => updateArbitre({ name: e.target.value })}
          className={inputClass}
        />
      </div>

      <div className="space-y-2">
        <SectionLabel icon={FileText}>{t("setup.arbitrePrompt")}</SectionLabel>
        <PersonaEditor
          systemPrompt={arbitre.systemPrompt}
          profileType="arbitre"
          discussionLanguage={discussionLanguage}
          onChange={(prompt) => updateArbitre({ systemPrompt: prompt })}
        />
      </div>

      {/* Save as custom profile */}
      {isCustomConfig && arbitre.name.trim() && arbitre.systemPrompt.trim() && (
        !showSaveForm ? (
          <button
            onClick={() => setShowSaveForm(true)}
            className="flex items-center gap-1.5 text-sm text-primary hover:text-primary/80"
          >
            <Save className="h-4 w-4" />
            {t("setup.saveArbitreProfile")}
          </button>
        ) : (
          <div className="flex flex-wrap items-end gap-2 rounded-md border border-border bg-card p-3">
            <div className="min-w-40 flex-1 space-y-1">
              <label className="text-xs text-muted-foreground">{t("setup.arbitreProfilePersonality")}</label>
              <input
                type="text"
                value={savePersonality}
                onChange={(e) => setSavePersonality(e.target.value)}
                placeholder={t("setup.personalityPlaceholder")}
                className={`${inputClass} py-1.5`}
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
        )
      )}

      <button
        onClick={() => setShowLlm(!showLlm)}
        className="flex w-full items-center gap-1.5 border-b border-border pb-2 text-sm font-medium text-foreground hover:text-foreground/80"
      >
        <Sliders className="h-4 w-4 text-primary" />
        {t("setup.llmParams")}
        <span className="ml-auto">
          {showLlm ? <ChevronUp className="h-4 w-4 text-muted-foreground" /> : <ChevronDown className="h-4 w-4 text-muted-foreground" />}
        </span>
      </button>
      {showLlm && <LlmParamsForm params={arbitre.llmParams} onChange={updateArbitreLlm} />}
    </div>
  );
}
