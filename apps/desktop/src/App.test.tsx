import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { OVERLAY_MODE_EVENT } from "./features/overlay/useOverlayMode";

describe("App", () => {
  beforeEach(() => {
    mockIPC(() => undefined, { shouldMockEvents: true });
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
});
