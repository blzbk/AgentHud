import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AGENTS_COMMAND, AGENTS_EVENT, type ProviderAgents, useAgents } from "./useAgents";

const UNKNOWN: ProviderAgents[] = [
  { provider: "codex", status: "unknown", agents: [] },
  { provider: "claude", status: "unknown", agents: [] },
];

const ACTIVE: ProviderAgents[] = [
  { provider: "codex", status: "offline", agents: [] },
  {
    provider: "claude",
    status: "active",
    agents: [{ sessionId: "a1", project: "project", state: "working", lastWrite: 1_793_831_988 }],
  },
];

function mockNativeAgents(agents: unknown) {
  mockIPC((command) => (command === AGENTS_COMMAND ? agents : undefined), {
    shouldMockEvents: true,
  });
}

describe("useAgents", () => {
  it("starts with every provider unknown", () => {
    mockNativeAgents(new Promise(() => undefined));
    const { result } = renderHook(() => useAgents());

    expect(result.current).toEqual(UNKNOWN);
  });

  it("shows the native agents read on load", async () => {
    mockNativeAgents(ACTIVE);
    const { result } = renderHook(() => useAgents());

    await waitFor(() => expect(result.current).toEqual(ACTIVE));
  });

  it("ignores a load answer that is not a list", async () => {
    mockNativeAgents(undefined);
    const { result } = renderHook(() => useAgents());

    await act(() => Promise.resolve());

    expect(result.current).toEqual(UNKNOWN);
  });

  it("keeps an emitted change over a slower load read", async () => {
    let resolveLoad: (agents: ProviderAgents[]) => void = () => undefined;
    mockIPC(
      (command) =>
        command === AGENTS_COMMAND
          ? new Promise<ProviderAgents[]>((resolve) => {
              resolveLoad = resolve;
            })
          : undefined,
      { shouldMockEvents: true },
    );
    const { result } = renderHook(() => useAgents());

    await act(() => emit(AGENTS_EVENT, ACTIVE));
    await act(async () => {
      resolveLoad(UNKNOWN);
      await Promise.resolve();
    });

    expect(result.current).toEqual(ACTIVE);
  });
});
