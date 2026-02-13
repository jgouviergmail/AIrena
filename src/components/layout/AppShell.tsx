import { Outlet } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { ToastContainer } from "@/components/shared/ToastContainer";

export function AppShell() {
  return (
    <div className="flex h-screen overflow-hidden bg-background">
      <Sidebar />
      <main className="flex flex-1 flex-col overflow-hidden">
        <Outlet />
      </main>
      <ToastContainer />
    </div>
  );
}
