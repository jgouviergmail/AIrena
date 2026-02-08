import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown } from "lucide-react";
import { EMOJI_PALETTE } from "@/lib/profile-emoji";

interface EmojiPickerProps {
  value: string | undefined;
  autoEmoji: string;
  onSelect: (emoji: string | undefined) => void;
}

export function EmojiPicker({ value, autoEmoji, onSelect }: EmojiPickerProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  const displayed = value ?? autoEmoji;

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex h-7 w-9 items-center justify-center rounded border border-border text-base transition-colors hover:bg-accent"
        title={value ? displayed : `${displayed} (${t("setup.emojiAuto")})`}
      >
        <span className="text-sm">{displayed}</span>
        <ChevronDown className="ml-px h-2.5 w-2.5 text-muted-foreground" />
      </button>

      {open && (
        <div className="absolute left-0 top-full z-50 mt-1 w-[304px] rounded-lg border border-border bg-popover p-3 shadow-lg">
          {/* Auto button */}
          <button
            type="button"
            onClick={() => { onSelect(undefined); setOpen(false); }}
            className={`mb-2 w-full rounded px-2 py-1.5 text-left text-sm transition-colors ${
              !value
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:bg-accent"
            }`}
          >
            {autoEmoji} {t("setup.emojiAuto")}
          </button>

          {/* Emoji grid */}
          <div className="grid grid-cols-7 gap-1">
            {EMOJI_PALETTE.map((emoji) => (
              <button
                key={emoji}
                type="button"
                onClick={() => { onSelect(emoji); setOpen(false); }}
                className={`flex h-9 w-9 items-center justify-center rounded text-lg transition-colors hover:bg-accent ${
                  value === emoji ? "bg-primary/10 ring-1 ring-primary" : ""
                }`}
              >
                {emoji}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
