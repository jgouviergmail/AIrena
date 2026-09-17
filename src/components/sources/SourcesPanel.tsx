import { useTranslation } from "react-i18next";
import { useArenaStore } from "@/stores/useArenaStore";
import { SourcesList } from "./SourcesList";

/** Arena side-panel tab: every reference injected so far, live. */
export function SourcesPanel() {
  const { t } = useTranslation();
  const sources = useArenaStore((s) => s.sources);
  const cited = sources.filter((s) => s.cited).length;
  return (
    <>
      <div className="flex shrink-0 items-center gap-2 border-b border-border px-3 py-1.5 text-[10px] text-muted-foreground">
        <span>{t("sources.count", { count: sources.length })}</span>
        {cited > 0 && <span>· {t("sources.citedCount", { count: cited })}</span>}
      </div>
      <SourcesList sources={sources} />
    </>
  );
}
