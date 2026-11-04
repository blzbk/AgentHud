import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export type OverlayMode = "click-through" | "interactive";

/** Emitted by the native side each time the overlay mode changes. */
export const OVERLAY_MODE_EVENT = "overlay-mode";

/** Current overlay mode. The native side starts click-through. */
export function useOverlayMode(): OverlayMode {
  const [mode, setMode] = useState<OverlayMode>("click-through");

  useEffect(() => {
    const unlisten = listen<OverlayMode>(OVERLAY_MODE_EVENT, (event) => {
      setMode(event.payload);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  return mode;
}
