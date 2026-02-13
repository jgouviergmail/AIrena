import { create } from "zustand";

export type ToastLevel = "success" | "error" | "warning" | "info";

export interface Toast {
  id: string;
  level: ToastLevel;
  message: string;
  detail?: string;
  createdAt: number;
}

interface ToastState {
  toasts: Toast[];
  addToast: (level: ToastLevel, message: string, detail?: string) => void;
  removeToast: (id: string) => void;
}

let nextId = 0;

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],

  addToast: (level, message, detail) => {
    const id = `toast-${++nextId}`;
    const toast: Toast = { id, level, message, detail, createdAt: Date.now() };
    set((s) => ({ toasts: [...s.toasts, toast] }));

    // Auto-dismiss: errors stay 8s, others 4s
    const delay = level === "error" ? 8000 : 4000;
    setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, delay);
  },

  removeToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

/** Shorthand to show a toast from anywhere (outside React components) */
export const toast = {
  success: (message: string, detail?: string) =>
    useToastStore.getState().addToast("success", message, detail),
  error: (message: string, detail?: string) =>
    useToastStore.getState().addToast("error", message, detail),
  warning: (message: string, detail?: string) =>
    useToastStore.getState().addToast("warning", message, detail),
  info: (message: string, detail?: string) =>
    useToastStore.getState().addToast("info", message, detail),
};
