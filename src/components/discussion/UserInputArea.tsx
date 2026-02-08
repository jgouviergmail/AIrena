import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Send, X } from "lucide-react";
import * as api from "@/lib/tauri-api";
import { useArenaStore } from "@/stores/useArenaStore";

export function UserInputArea({
  timeoutSecs,
}: {
  timeoutSecs: number;
}) {
  const { t } = useTranslation();
  const [message, setMessage] = useState("");
  const [remaining, setRemaining] = useState(timeoutSecs);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
    const interval = setInterval(() => {
      setRemaining((prev) => {
        if (prev <= 1) {
          clearInterval(interval);
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const handleSubmit = async () => {
    if (!message.trim()) return;
    try {
      await api.submitUserMessage(message.trim());
      setMessage("");
      useArenaStore.setState({ userTurnActive: false });
    } catch (e) {
      console.error("Failed to submit user message:", e);
    }
  };

  const handleSkip = async () => {
    try {
      await api.skipUserTurn();
      useArenaStore.setState({ userTurnActive: false });
    } catch (e) {
      console.error("Failed to skip turn:", e);
    }
  };

  return (
    <div className="rounded-lg border-2 border-primary bg-primary/5 p-4">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-sm font-medium text-primary">
          {t("arena.userTurnReady", { seconds: remaining })}
        </p>
        <button
          onClick={handleSkip}
          className="flex items-center gap-1 rounded px-2 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-foreground"
        >
          <X className="h-3 w-3" />
          {t("arena.skipTurn")}
        </button>
      </div>

      <div className="flex gap-2">
        <textarea
          ref={inputRef}
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              handleSubmit();
            }
          }}
          placeholder={t("arena.messagePlaceholder")}
          rows={2}
          className="flex-1 rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        />
        <button
          onClick={handleSubmit}
          disabled={!message.trim()}
          className="flex h-10 w-10 items-center justify-center self-end rounded-md bg-primary text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-30"
        >
          <Send className="h-4 w-4" />
        </button>
      </div>

      {/* Timer bar */}
      <div className="mt-2 h-1 w-full rounded-full bg-muted">
        <div
          className="h-full rounded-full bg-primary transition-all duration-1000"
          style={{
            width: `${(remaining / timeoutSecs) * 100}%`,
          }}
        />
      </div>
    </div>
  );
}
