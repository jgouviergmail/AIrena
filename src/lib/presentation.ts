// Projection mode (v1.18): sidebars hidden and the window fullscreen. The
// fullscreen call only exists inside Tauri; in a plain browser it fails
// quietly and the layout still switches.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useUiStore } from "@/stores/useUiStore";
import { logger } from "@/lib/logger";

/** Enter or leave the projection mode. */
export async function setPresentation(on: boolean): Promise<void> {
  useUiStore.getState().setPresentationMode(on);
  try {
    await getCurrentWindow().setFullscreen(on);
  } catch (e) {
    logger.warn("ui", `Fullscreen unavailable: ${e instanceof Error ? e.message : String(e)}`);
  }
}
