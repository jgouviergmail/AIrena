import { create } from "zustand";

const STORAGE_KEY = "airena.ui";

/** Presentation preferences (per device, never sent to the engine). */
export interface UiPrefs {
  /** The stage (arc of avatars) above the feed */
  stageVisible: boolean;
}

interface UiState extends UiPrefs {
  /** Projection: sidebars hidden, window fullscreen */
  presentationMode: boolean;
  /** Turn the keyboard navigation last scrolled to (null = follow the live turn) */
  viewedTurn: number | null;
  setPresentationMode: (on: boolean) => void;
  toggleStage: () => void;
}

const DEFAULT_PREFS: UiPrefs = { stageVisible: true };

/** Saved preferences, defaults when missing or malformed. */
export function readPrefs(): UiPrefs {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_PREFS;
    const parsed = JSON.parse(raw) as Partial<UiPrefs>;
    return { stageVisible: typeof parsed.stageVisible === "boolean" ? parsed.stageVisible : DEFAULT_PREFS.stageVisible };
  } catch {
    return DEFAULT_PREFS;
  }
}

function persist(prefs: UiPrefs) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(prefs));
  } catch { /* storage unavailable: preferences live for the session */ }
}

const prefsOf = (s: UiState): UiPrefs => ({ stageVisible: s.stageVisible });

export const useUiStore = create<UiState>((set, get) => ({
  ...readPrefs(),
  presentationMode: false,
  viewedTurn: null,

  setPresentationMode: (on) => set({ presentationMode: on }),

  toggleStage: () => {
    set((s) => ({ stageVisible: !s.stageVisible }));
    persist(prefsOf(get()));
  },
}));
