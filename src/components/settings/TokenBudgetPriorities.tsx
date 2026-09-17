import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, Layers, RotateCcw } from "lucide-react";
import { useSettingsStore } from "@/stores/useSettingsStore";
import { CONFIGURABLE_BUDGET_SECTIONS, LATER_BUDGET_SECTIONS } from "@/lib/types";
import type { SectionPriority } from "@/lib/types";
import { Explainer, Field, Section } from "./SettingsPrimitives";

/** First rank of the user-configurable sections (ranks 1–3 are fixed by the backend). */
const FIRST_CONFIGURABLE_RANK = 4;

/**
 * Parse the stored JSON, falling back to the default order when incomplete.
 * Sections added after v1.16 that the saved order lacks are appended after the
 * user's sections (mirrors the backend's `apply_default_bounds`).
 */
export function parsePriorities(value: string): SectionPriority[] {
  try {
    if (value) {
      const parsed: SectionPriority[] = JSON.parse(value);
      // Filter to CONFIGURABLE sections only (document sections are auto-managed by backend)
      const configurable = parsed.filter((p) => CONFIGURABLE_BUDGET_SECTIONS.includes(p.section));
      const present = new Set(configurable.map((p) => p.section));
      if (CONFIGURABLE_BUDGET_SECTIONS.every((s) => present.has(s) || LATER_BUDGET_SECTIONS.includes(s))) {
        const ordered = [...configurable].sort((a, b) => a.rank - b.rank);
        for (const section of LATER_BUDGET_SECTIONS) {
          if (!present.has(section)) ordered.push({ section, rank: 0, floor: 0, ceiling: 0 });
        }
        return ordered.map((s, i) => ({ ...s, rank: i + FIRST_CONFIGURABLE_RANK }));
      }
    }
  } catch { /* fall through to defaults */ }
  return CONFIGURABLE_BUDGET_SECTIONS.map((section, i) => ({
    section,
    rank: i + FIRST_CONFIGURABLE_RANK,
    floor: 0,
    ceiling: 0,
  }));
}

/** Swap two entries and re-number ranks from FIRST_CONFIGURABLE_RANK. */
export function moveSection(sections: SectionPriority[], from: number, to: number): SectionPriority[] {
  if (to < 0 || to >= sections.length || from === to) return sections;
  const updated = [...sections];
  [updated[from], updated[to]] = [updated[to], updated[from]];
  return updated.map((s, i) => ({ ...s, rank: i + FIRST_CONFIGURABLE_RANK }));
}

/** Provider-agnostic ordering of the prompt sections competing for context. */
export function TokenBudgetPrioritiesSettings() {
  const { t } = useTranslation();
  const value = useSettingsStore((s) => s.settings.tokenBudgetPriorities);
  const updateSettings = useSettingsStore((s) => s.updateSettings);
  const sections = parsePriorities(value);
  const commit = (next: SectionPriority[]) => updateSettings({ tokenBudgetPriorities: JSON.stringify(next) });

  return (
    <Section title={t("settings.tokenBudget")} icon={Layers}>
      <Field label={t("settings.tokenBudget")}>
        <Explainer>{t("settings.tokenBudgetDesc")}</Explainer>
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground">{t("settings.tokenBudgetDragHint")}</p>
          <div className="space-y-1">
            {sections.map((s, i) => (
              <div key={s.section} className="flex items-center gap-2 rounded-md border border-border bg-background px-3 py-1.5">
                <span className="w-5 text-center text-xs font-mono text-muted-foreground">{s.rank}</span>
                <span className="flex-1 text-sm text-foreground">{t(`setup.budgetSection_${s.section}`)}</span>
                <button
                  onClick={() => commit(moveSection(sections, i, i - 1))}
                  disabled={i === 0}
                  className="rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-20"
                  aria-label={t("settings.moveUp")}
                >
                  <ArrowUp className="h-3.5 w-3.5" />
                </button>
                <button
                  onClick={() => commit(moveSection(sections, i, i + 1))}
                  disabled={i === sections.length - 1}
                  className="rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-20"
                  aria-label={t("settings.moveDown")}
                >
                  <ArrowDown className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
          </div>
          <button
            onClick={() => updateSettings({ tokenBudgetPriorities: "" })}
            className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground"
          >
            <RotateCcw className="h-3 w-3" />
            {t("settings.tokenBudgetReset")}
          </button>
        </div>
      </Field>
    </Section>
  );
}
