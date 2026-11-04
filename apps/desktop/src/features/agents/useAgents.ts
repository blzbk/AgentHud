import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { PROVIDERS, type Provider } from "../usage/useUsage";

/*
 * Wire types of the `agents` command and event, written by hand to mirror
 * `AgentReport` in `crates/application/src/agent_report.rs`.
 * Times are seconds since the Unix epoch.
 */

export type AgentState = "waiting" | "working" | "idle" | "unknown";

export type Agent = {
  sessionId: string;
  project?: string;
  state: AgentState;
  lastWrite: number;
};

export type ProviderAgents = {
  provider: Provider;
  /** `active` lists the agents; `offline` means no session is running. */
  status: "active" | "offline" | "unknown";
  agents: Agent[];
};

/** Emitted by the native side each time an agent changes. */
export const AGENTS_EVENT = "agents";

/** Native command returning the current agents, read once on load. */
export const AGENTS_COMMAND = "agents";

const INITIAL_AGENTS: ProviderAgents[] = PROVIDERS.map((provider) => ({
  provider,
  status: "unknown",
  agents: [],
}));

/**
 * Agents of each provider. Reads the native agents on load, then follows
 * each change the native side emits, like `useUsage`.
 */
export function useAgents(): ProviderAgents[] {
  const [agents, setAgents] = useState<ProviderAgents[]>(INITIAL_AGENTS);

  useEffect(() => {
    let changedSinceLoad = false;
    const unlisten = listen<ProviderAgents[]>(AGENTS_EVENT, (event) => {
      changedSinceLoad = true;
      setAgents(event.payload);
    });
    invoke<ProviderAgents[]>(AGENTS_COMMAND)
      .then((current) => {
        if (!changedSinceLoad && Array.isArray(current)) setAgents(current);
      })
      .catch((error: unknown) => {
        console.error("cannot read the agents, keeping unknown", error);
      });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  return agents;
}
