import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

/*
 * Wire types of the `usage` command and event, written by hand to mirror
 * `UsageReport` in `crates/application/src/usage_report.rs`.
 * Times are seconds since the Unix epoch.
 */

export type WindowUsage = {
  usedPercent: number;
  resetsAt?: number;
};

export type ProviderUsage = {
  provider: "codex" | "claude";
  status: "known" | "unknown";
  fiveHours?: WindowUsage;
  weekly?: WindowUsage;
  observedAt?: number;
};

/** Emitted by the native side each time a provider's usage changes. */
export const USAGE_EVENT = "usage";

/** Native command returning the current usage, read once on load. */
export const USAGE_COMMAND = "usage";

const INITIAL_USAGE: ProviderUsage[] = [{ provider: "codex", status: "unknown" }];

/**
 * Usage of each provider. Reads the native usage on load, so a reloaded page
 * shows the truth, then follows each change the native side emits.
 */
export function useUsage(): ProviderUsage[] {
  const [usage, setUsage] = useState<ProviderUsage[]>(INITIAL_USAGE);

  useEffect(() => {
    let changedSinceLoad = false;
    const unlisten = listen<ProviderUsage[]>(USAGE_EVENT, (event) => {
      changedSinceLoad = true;
      setUsage(event.payload);
    });
    invoke<ProviderUsage[]>(USAGE_COMMAND)
      .then((current) => {
        if (!changedSinceLoad && Array.isArray(current)) setUsage(current);
      })
      .catch((error: unknown) => {
        console.error("cannot read the usage, keeping unknown", error);
      });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  return usage;
}
