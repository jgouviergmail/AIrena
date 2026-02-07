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

function Loading() {
  return (
    <div className="flex h-full items-center justify-center">
      <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
    </div>
  );
}

function AppInit({ children }: { children: React.ReactNode }) {
  const hydrate = useSettingsStore((s) => s.hydrate);
  useEffect(() => {
    hydrate();
  }, [hydrate]);
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
