import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

/*
 * Wire types of the `usage` command and event, written by hand to mirror
 * `UsageReport` in `crates/application/src/usage_report.rs`.
 * Times are seconds since the Unix epoch.
 */

/** A window known in percent (Codex) or in tokens (Claude). */
export type WindowUsage = {
  usedPercent?: number;
  usedTokens?: number;
  resetsAt?: number;
  /** The window bounds are guessed, not published by the provider. */
  estimated?: boolean;
};

export type Provider = "codex" | "claude";

/** Where the usage comes from, mirroring `Origin` in `crates/domain`. */
export type UsageSource = "sessions" | "app-server" | "statusline" | "oauth" | "estimate";

export type ProviderUsage = {
  provider: Provider;
  status: "known" | "unknown";
  fiveHours?: WindowUsage;
  weekly?: WindowUsage;
  observedAt?: number;
  source?: UsageSource;
  /** A polled source of the provider needs the user to log in again. */
  authExpired?: boolean;
};

/** Providers in display order. */
export const PROVIDERS: readonly Provider[] = ["codex", "claude"];

/** Emitted by the native side each time a provider's usage changes. */
export const USAGE_EVENT = "usage";

/** Native command returning the current usage, read once on load. */
export const USAGE_COMMAND = "usage";

const INITIAL_USAGE: ProviderUsage[] = PROVIDERS.map((provider) => ({
  provider,
  status: "unknown",
}));

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
