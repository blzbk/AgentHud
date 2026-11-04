import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type OverlayMode = "click-through" | "interactive";

/** Emitted by the native side each time the overlay mode changes. */
export const OVERLAY_MODE_EVENT = "overlay-mode";

/** Native command returning the current mode, read once on load. */
export const OVERLAY_MODE_COMMAND = "overlay_mode";

/**
 * Current overlay mode. Reads the native mode on load, so a reloaded page
 * shows the truth, then follows each change the native side emits.
 */
export function useOverlayMode(): OverlayMode {
  const [mode, setMode] = useState<OverlayMode>("click-through");

  useEffect(() => {
    let changedSinceLoad = false;
    const unlisten = listen<OverlayMode>(OVERLAY_MODE_EVENT, (event) => {
      changedSinceLoad = true;
      setMode(event.payload);
    });
    void invoke<OverlayMode>(OVERLAY_MODE_COMMAND).then((current) => {
      if (!changedSinceLoad) setMode(current);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  return mode;
}
