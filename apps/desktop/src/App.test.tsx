import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { OVERLAY_MODE_COMMAND, OVERLAY_MODE_EVENT } from "./features/overlay/useOverlayMode";
import { type ProviderUsage, USAGE_COMMAND, USAGE_EVENT } from "./features/usage/useUsage";

const NATIVE_ANSWERS: Record<string, unknown> = {
  [OVERLAY_MODE_COMMAND]: "click-through",
  [USAGE_COMMAND]: [{ provider: "codex", status: "unknown" }],
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
});
