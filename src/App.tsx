import { lazy, Suspense, useEffect } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Loader2 } from "lucide-react";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { ErrorBoundary } from "@/components/common/ErrorBoundary";
import { AppShell } from "@/components/layout/AppShell";
import { useSettingsStore } from "@/stores/useSettingsStore";

const HomePage = lazy(() => import("@/pages/HomePage"));
const SettingsPage = lazy(() => import("@/pages/SettingsPage"));
const SetupPage = lazy(() => import("@/pages/SetupPage"));
const ArenaPage = lazy(() => import("@/pages/ArenaPage"));
const SummaryPage = lazy(() => import("@/pages/SummaryPage"));
const HistoryPage = lazy(() => import("@/pages/HistoryPage"));
const HistoryDetailPage = lazy(() => import("@/pages/HistoryDetailPage"));

function Loading() {
  return (
    <div className="flex h-full items-center justify-center">
      <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
    </div>
  );
}

function AppInit({ children }: { children: React.ReactNode }) {
  const hydrate = useSettingsStore((s) => s.hydrate);
  const loading = useSettingsStore((s) => s.loading);
  const ollamaModel = useSettingsStore((s) => s.settings.ollamaModel);
  const ollamaInitialized = useSettingsStore((s) => s.ollamaInitialized);
  const initializeOllama = useSettingsStore((s) => s.initializeOllama);

  useEffect(() => {
    hydrate();
  }, [hydrate]);

  // After hydration, initialize Ollama (unload VRAM → detect → recommend → preload)
  useEffect(() => {
    if (!loading && ollamaModel && !ollamaInitialized) {
      initializeOllama();
    }
  }, [loading, ollamaModel, ollamaInitialized, initializeOllama]);

  return <>{children}</>;
}

function App() {
  return (
    <ErrorBoundary>
      <ThemeProvider>
        <BrowserRouter>
          <AppInit>
            <Suspense fallback={<Loading />}>
              <Routes>
                <Route element={<AppShell />}>
                  <Route path="/" element={<HomePage />} />
                  <Route path="/settings" element={<SettingsPage />} />
                  <Route path="/setup" element={<SetupPage />} />
                  <Route path="/arena" element={<ArenaPage />} />
                  <Route path="/summary" element={<SummaryPage />} />
                  <Route path="/history" element={<HistoryPage />} />
                  <Route path="/history/:id" element={<HistoryDetailPage />} />
                </Route>
              </Routes>
            </Suspense>
          </AppInit>
        </BrowserRouter>
      </ThemeProvider>
    </ErrorBoundary>
  );
}

export default App;
