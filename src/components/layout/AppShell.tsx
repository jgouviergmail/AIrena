import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { ToastContainer } from "@/components/shared/ToastContainer";
import { useUiStore } from "@/stores/useUiStore";
import { useAudioSettingsSync } from "@/hooks/useArenaAudio";

export function AppShell() {
  const presentation = useUiStore((s) => s.presentationMode);
  useAudioSettingsSync();
  return (
    <div className="flex h-screen overflow-hidden bg-background">
      {!presentation && <Sidebar />}
      <main className="flex flex-1 flex-col overflow-hidden">
        <Outlet />
      </main>
      <ToastContainer />
    </div>
  );
}
