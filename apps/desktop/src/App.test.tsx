import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { AGENTS_COMMAND, AGENTS_EVENT, type ProviderAgents } from "./features/agents/useAgents";
import { OVERLAY_MODE_COMMAND, OVERLAY_MODE_EVENT } from "./features/overlay/useOverlayMode";
import { type ProviderUsage, USAGE_COMMAND, USAGE_EVENT } from "./features/usage/useUsage";

const NATIVE_ANSWERS: Record<string, unknown> = {
  [OVERLAY_MODE_COMMAND]: "click-through",
  [USAGE_COMMAND]: [{ provider: "codex", status: "unknown" }],
  [AGENTS_COMMAND]: [{ provider: "codex", status: "unknown", agents: [] }],
};

describe("App", () => {
  beforeEach(() => {
    mockIPC((command) => NATIVE_ANSWERS[command], { shouldMockEvents: true });
  });

  it("shows the AgentHud title", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "AgentHud" })).toBeInTheDocument();
  });

  it("shows the click-through mode at startup", () => {
    render(<App />);

    expect(screen.getByRole("status")).toHaveTextContent("click-through");
  });

  it("shows the interactive mode once the native side switches to it", async () => {
    render(<App />);

    await act(() => emit(OVERLAY_MODE_EVENT, "interactive"));

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("interactif"));
  });

  it("lets the title bar move the window only in interactive mode", async () => {
    render(<App />);
    const header = screen.getByRole("heading", { name: "AgentHud" }).closest("header");

    expect(header).not.toHaveAttribute("data-tauri-drag-region");

    await act(() => emit(OVERLAY_MODE_EVENT, "interactive"));

    await waitFor(() => expect(header).toHaveAttribute("data-tauri-drag-region", "deep"));
  });

  it("shows the Codex card as unknown until its usage is known", () => {
    render(<App />);

    const card = screen.getByRole("region", { name: "Codex" });
    expect(card).toHaveTextContent("unknown");
  });

  it("shows the Codex usage once the native side sends it", async () => {
    const usage: ProviderUsage[] = [
      { provider: "codex", status: "known", fiveHours: { usedPercent: 12 }, weekly: { usedPercent: 51 } },
    ];
    render(<App />);

    await act(() => emit(USAGE_EVENT, usage));

    await waitFor(() =>
      expect(screen.getByRole("meter", { name: "hebdo" })).toHaveAttribute("aria-valuenow", "51"),
    );
  });

  it("shows one card per provider, Codex then Claude", () => {
    render(<App />);

    expect(screen.getAllByRole("region").map((card) => card.getAttribute("aria-label"))).toEqual([
      "Codex",
      "Claude",
    ]);
  });

  it("shows each provider's usage and agents once the native side sends them", async () => {
    const usage: ProviderUsage[] = [
      { provider: "codex", status: "known", fiveHours: { usedPercent: 12 }, weekly: { usedPercent: 51 } },
      {
        provider: "claude",
        status: "known",
        fiveHours: { usedTokens: 1_200_000, resetsAt: Date.now() / 1000 + 3600, estimated: true },
      },
    ];
    const agents: ProviderAgents[] = [
      { provider: "codex", status: "offline", agents: [] },
      {
        provider: "claude",
        status: "active",
        agents: [{ sessionId: "a1", project: "AgentHud", state: "waiting", lastWrite: 0 }],
      },
    ];
    render(<App />);

    await act(async () => {
      await emit(USAGE_EVENT, usage);
      await emit(AGENTS_EVENT, agents);
    });

    const claude = screen.getByRole("region", { name: "Claude" });
    const codex = screen.getByRole("region", { name: "Codex" });
    await waitFor(() => expect(claude).toHaveTextContent("1,2 M tokens"));
    expect(claude).toHaveTextContent("estimé");
    expect(within(claude).getByRole("listitem")).toHaveTextContent("waitingAgentHud");
    expect(codex).toHaveTextContent("offline");
    expect(within(codex).queryByRole("list")).not.toBeInTheDocument();
  });
});
