import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, ChevronDown, ChevronRight, ChevronUp, GripVertical, Heart, Plus, RotateCcw, Save, Trash2, Users } from "lucide-react";
import { LlmParamsForm } from "@/components/setup/LlmParamsForm";
import { PersonaEditor } from "@/components/setup/PersonaEditor";
import { EmojiPicker } from "@/components/setup/EmojiPicker";
import { getProfileEmoji } from "@/lib/profile-emoji";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { cn } from "@/lib/utils";
import { DEFAULT_LLM_PARAMS } from "@/lib/types";
import type { GladIAteurConfig, PredefinedProfile } from "@/lib/types";
import { SectionLabel } from "./shared";

const CATEGORY_ORDER = ["personnel", "experts", "imaginaires", "personnalites", "metiers", "autres"] as const;

const newGladiateurId = () => `glad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

export function StepGladiateurs() {
  const { t } = useTranslation();
  const gladiateurs = useSetupStore((s) => s.gladiateurs);
  const addGladiateur = useSetupStore((s) => s.addGladiateur);
  const removeGladiateur = useSetupStore((s) => s.removeGladiateur);
  const updateGladiateur = useSetupStore((s) => s.updateGladiateur);
  const updateGladiateurLlm = useSetupStore((s) => s.updateGladiateurLlm);
  const reorderGladiateurs = useSetupStore((s) => s.reorderGladiateurs);
  const profiles = useSettingsStore((s) => s.profiles);
  const emotionDriven = useSettingsStore((s) => s.settings.emotionDriven);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const saveProfile = useSettingsStore((s) => s.saveProfile);
  const deleteProfile = useSettingsStore((s) => s.deleteProfile);
  const discussionLanguage = useSetupStore((s) => s.discussionLanguage);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [collapsedCats, setCollapsedCats] = useState<Set<string>>(() => new Set(CATEGORY_ORDER));
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

  const addFromProfile = (profile: PredefinedProfile) => {
    addGladiateur({
      id: newGladiateurId(),
      name: t(`profiles.${profile.id}.name`, { defaultValue: profile.name }),
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: t(`profiles.${profile.id}.systemPrompt`, { defaultValue: profile.systemPrompt }),
      llmParams: { ...DEFAULT_LLM_PARAMS },
      sourceProfileId: profile.id,
      initialEmotions: profile.initialEmotions,
    });
  };

  const addEmpty = () => {
    addGladiateur({
      id: newGladiateurId(),
      name: "",
      interventionNumber: gladiateurs.length + 1,
      systemPrompt: "",
      llmParams: { ...DEFAULT_LLM_PARAMS },
    });
  };

  const handleToggleEmotionDriven = () => {
    updateSettings({ emotionDriven: !emotionDriven });
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

  return (
    <div className="space-y-6">
      {/* Emotion-driven behavior toggle */}
      <div className="space-y-3">
        <SectionLabel icon={Heart}>{t("settings.emotionDriven")}</SectionLabel>
        <div className="flex items-center gap-3">
          <button
            onClick={handleToggleEmotionDriven}
            role="switch"
            aria-checked={emotionDriven}
            className={cn(
              "relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors",
              emotionDriven ? "bg-primary" : "bg-muted",
            )}
          >
            <span
              className={cn(
                "pointer-events-none inline-block h-5 w-5 rounded-full bg-background shadow-lg ring-0 transition-transform",
                emotionDriven ? "translate-x-5" : "translate-x-0",
              )}
            />
          </button>
          <span className="text-sm text-muted-foreground">{t("settings.emotionDrivenDesc")}</span>
        </div>
      </div>

      {/* Profile picker grouped by category */}
      <div className="space-y-3">
        <SectionLabel icon={Users}>{t("setup.selectProfile")}</SectionLabel>
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
                {collapsed ? <ChevronRight className="h-3.5 w-3.5" /> : <ChevronDown className="h-3.5 w-3.5" />}
                {t(`setup.category_${group.category}`)}
                <span className="font-normal normal-case">({group.profiles.length})</span>
              </button>
              {!collapsed && (
                <div className="flex flex-wrap gap-2">
                  {group.profiles.map((p) => (
                    <div key={p.id} className="group/profile relative">
                      <button
                        onClick={() => addFromProfile(p)}
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
            onClick={addEmpty}
            className="rounded-md border border-dashed border-border px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <Plus className="mr-1.5 inline h-3.5 w-3.5" />
            {t("setup.addGladiateur")}
          </button>
        </div>
      </div>

      {gladiateurs.length === 0 && (
        <p className="py-8 text-center text-sm text-muted-foreground">{t("setup.minGladiateurs")}</p>
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
            <div className="flex items-center justify-between gap-2">
              <div className="flex min-w-0 items-center gap-3">
                <GripVertical
                  className="h-4 w-4 shrink-0 cursor-grab text-muted-foreground active:cursor-grabbing"
                  onPointerDown={(e) => {
                    e.preventDefault();
                    dragFrom.current = idx;
                    setIsDragging(true);
                  }}
                />
                <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
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
                  onChange={(e) => updateGladiateur(g.id, { name: e.target.value })}
                  placeholder={t("setup.gladiateurName")}
                  className="min-w-0 border-none bg-transparent text-sm font-medium text-foreground placeholder:text-muted-foreground focus:outline-none"
                />
              </div>
              <div className="flex shrink-0 items-center gap-1">
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
                  onClick={() => setExpandedId(expandedId === g.id ? null : g.id)}
                  className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
                >
                  {expandedId === g.id ? <ChevronUp className="h-4 w-4" /> : <ChevronDown className="h-4 w-4" />}
                </button>
                {g.name.trim() && g.systemPrompt.trim() && (
                  <button
                    onClick={() => handleSaveGladiateur(g)}
                    title={t("setup.saveGladiateur")}
                    className={cn(
                      "rounded p-1 transition-colors",
                      savedIds.has(g.id) ? "text-green-500" : "text-muted-foreground hover:bg-accent hover:text-foreground",
                    )}
                  >
                    {savedIds.has(g.id) ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
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
                  <label className="text-xs text-muted-foreground">{t("setup.gladiateurPrompt")}</label>
                  <PersonaEditor
                    systemPrompt={g.systemPrompt}
                    profileType="gladiateur"
                    discussionLanguage={discussionLanguage}
                    onChange={(prompt) => updateGladiateur(g.id, { systemPrompt: prompt })}
                  />
                </div>
                <div>
                  <label className="mb-2 block text-xs text-muted-foreground">{t("setup.llmParams")}</label>
                  <LlmParamsForm params={g.llmParams} onChange={(patch) => updateGladiateurLlm(g.id, patch)} />
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
