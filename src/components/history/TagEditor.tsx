import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, X } from "lucide-react";
import { normaliseTag, TAG_MAX_LENGTH } from "@/lib/history-filters";

/** Tag chips of one discussion with inline add / remove (stops the row's click). */
export function TagEditor({ tags, onChange }: { tags: string[]; onChange: (tags: string[]) => void }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState("");
  const [editing, setEditing] = useState(false);

  const commit = () => {
    const tag = normaliseTag(draft);
    setDraft("");
    setEditing(false);
    if (tag && !tags.includes(tag)) onChange([...tags, tag]);
  };

  return (
    <div className="flex flex-wrap items-center gap-1" onClick={(e) => e.stopPropagation()}>
      {tags.map((tag) => (
        <span key={tag} className="inline-flex items-center gap-0.5 rounded-full bg-primary/10 px-2 py-0.5 text-[11px] text-primary">
          #{tag}
          <button type="button" onClick={() => onChange(tags.filter((x) => x !== tag))} aria-label={t("history.removeTag", { tag })} className="rounded-full hover:bg-primary/20">
            <X className="h-3 w-3" />
          </button>
        </span>
      ))}
      {editing ? (
        <input
          autoFocus
          type="text"
          value={draft}
          maxLength={TAG_MAX_LENGTH}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit();
            if (e.key === "Escape") { setDraft(""); setEditing(false); }
          }}
          placeholder={t("history.addTag")}
          aria-label={t("history.addTag")}
          className="w-24 rounded-full border border-border bg-background px-2 py-0.5 text-[11px] text-foreground"
        />
      ) : (
        <button type="button" onClick={() => setEditing(true)} className="inline-flex items-center gap-0.5 rounded-full border border-dashed border-border px-2 py-0.5 text-[11px] text-muted-foreground hover:text-foreground">
          <Plus className="h-3 w-3" aria-hidden="true" />
          {t("history.addTag")}
        </button>
      )}
    </div>
  );
}
