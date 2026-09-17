import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookmarkPlus, FolderOpen, Loader2, Trash2 } from "lucide-react";
import * as api from "@/lib/tauri-api";
import { extractErrorMessage } from "@/lib/error-utils";
import { applyTemplate, newTemplate, parseTemplateConfig, templateFromSetup, TEMPLATE_NAME_MAX_CHARS, type ProfileText } from "@/lib/templates";
import type { DiscussionTemplate, PredefinedProfile } from "@/lib/types";
import { useSetupStore } from "@/stores/useSetupStore";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { toast } from "@/stores/useToastStore";
import { SectionLabel } from "./steps/shared";

const newGladiateurId = () => `glad-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;

/**
 * "Modèles de discussion": load a saved or builtin setup into the wizard, save
 * the current one. Builtin names are translated from the template id.
 */
export function TemplatePicker() {
  const { t } = useTranslation();
  const profiles = useSettingsStore((s) => s.profiles);
  const arbitreProfiles = useSettingsStore((s) => s.arbitreProfiles);
  const applyPatch = useSetupStore((s) => s.applyTemplatePatch);
  const updateArbitre = useSetupStore((s) => s.updateArbitre);
  const [templates, setTemplates] = useState<DiscussionTemplate[]>([]);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState("");
  const [saving, setSaving] = useState(false);
  const [name, setName] = useState("");

  const profileText: ProfileText = useCallback((p: PredefinedProfile) => ({
    name: t(`profiles.${p.id}.name`, { defaultValue: p.name }),
    systemPrompt: t(`profiles.${p.id}.systemPrompt`, { defaultValue: p.systemPrompt }),
  }), [t]);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setTemplates(await api.listDiscussionTemplates());
    } catch (e: unknown) {
      toast.error(t("templates.loadError"), extractErrorMessage(e));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { refresh(); }, [refresh]);

  const label = (tpl: DiscussionTemplate) => (tpl.builtin ? t(`templates.${tpl.id}`, { defaultValue: tpl.name }) : tpl.name);

  const load = () => {
    const tpl = templates.find((x) => x.id === selected);
    if (!tpl) return;
    const config = parseTemplateConfig(tpl.configJson);
    if (!config) {
      toast.error(t("templates.invalid"));
      return;
    }
    const applied = applyTemplate(config, profiles, arbitreProfiles, profileText, newGladiateurId);
    applyPatch(applied.patch);
    if (applied.arbitre) updateArbitre(applied.arbitre);
    if (applied.missing.length > 0) toast.error(t("templates.missingProfiles", { ids: applied.missing.join(", ") }));
    else toast.success(t("templates.loaded", { name: label(tpl) }));
  };

  const save = async () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    setSaving(true);
    try {
      const state = useSetupStore.getState();
      const config = templateFromSetup(state, arbitreProfiles, profileText);
      await api.saveDiscussionTemplate(newTemplate(trimmed, config));
      setName("");
      toast.success(t("templates.saved", { name: trimmed }));
      await refresh();
    } catch (e: unknown) {
      toast.error(t("templates.saveError"), extractErrorMessage(e));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    const tpl = templates.find((x) => x.id === selected);
    if (!tpl || tpl.builtin) return;
    if (!window.confirm(t("templates.deleteConfirm", { name: tpl.name }))) return;
    try {
      await api.deleteDiscussionTemplate(tpl.id);
      setSelected("");
      await refresh();
    } catch (e: unknown) {
      toast.error(t("templates.saveError"), extractErrorMessage(e));
    }
  };

  const current = templates.find((x) => x.id === selected);

  return (
    <div className="space-y-2">
      <SectionLabel icon={FolderOpen}>{t("templates.title")}</SectionLabel>
      <p className="text-xs text-muted-foreground">{t("templates.hint")}</p>
      <div className="flex flex-wrap items-center gap-2">
        <select
          value={selected}
          onChange={(e) => setSelected(e.target.value)}
          aria-label={t("templates.pick")}
          className="min-w-0 flex-1 rounded-md border border-border bg-background px-2 py-1.5 text-sm text-foreground"
        >
          <option value="">{loading ? "…" : t("templates.pick")}</option>
          {templates.map((tpl) => <option key={tpl.id} value={tpl.id}>{label(tpl)}{tpl.builtin ? "" : ` (${t("templates.mine")})`}</option>)}
        </select>
        <button type="button" onClick={load} disabled={!current} className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-50">
          <FolderOpen className="h-3.5 w-3.5" />
          {t("templates.load")}
        </button>
        {current && !current.builtin && (
          <button type="button" onClick={remove} title={t("templates.delete")} className="rounded-md p-1.5 text-muted-foreground hover:bg-destructive/10 hover:text-destructive">
            <Trash2 className="h-4 w-4" />
          </button>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") save(); }}
          placeholder={t("templates.namePlaceholder")}
          maxLength={TEMPLATE_NAME_MAX_CHARS}
          className="min-w-[10rem] flex-1 rounded-md border border-border bg-background px-2 py-1.5 text-sm text-foreground"
        />
        <button type="button" onClick={save} disabled={saving || !name.trim()} className="inline-flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-sm text-foreground hover:bg-accent disabled:opacity-50">
          {saving ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <BookmarkPlus className="h-3.5 w-3.5" />}
          {t("templates.saveCurrent")}
        </button>
      </div>
    </div>
  );
}
