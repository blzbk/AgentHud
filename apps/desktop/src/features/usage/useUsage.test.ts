import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { type ProviderUsage, USAGE_COMMAND, USAGE_EVENT, useUsage } from "./useUsage";

const KNOWN: ProviderUsage[] = [
  {
    provider: "codex",
    status: "known",
    fiveHours: { usedPercent: 12, resetsAt: 1_793_844_772 },
    weekly: { usedPercent: 51, resetsAt: 1_794_184_565 },
    observedAt: 1_793_831_988,
    source: "app-server",
    authExpired: true,
  },
];

const UNKNOWN: ProviderUsage[] = [
  { provider: "codex", status: "unknown" },
  { provider: "claude", status: "unknown" },
];

function mockNativeUsage(usage: unknown) {
  mockIPC((command) => (command === USAGE_COMMAND ? usage : undefined), {
    shouldMockEvents: true,
  });
}

describe("useUsage", () => {
  it("starts with every provider unknown", () => {
    mockNativeUsage(new Promise(() => undefined));
    const { result } = renderHook(() => useUsage());

    expect(result.current).toEqual(UNKNOWN);
  });

  it("shows the native usage read on load", async () => {
    mockNativeUsage(KNOWN);
    const { result } = renderHook(() => useUsage());

    await waitFor(() => expect(result.current).toEqual(KNOWN));
  });

  it("ignores a load answer that is not a list", async () => {
    mockNativeUsage(undefined);
    const { result } = renderHook(() => useUsage());

    await act(() => Promise.resolve());

    expect(result.current).toEqual(UNKNOWN);
  });

  it("follows the usage emitted by the native side", async () => {
    mockNativeUsage([{ provider: "codex", status: "unknown" }]);
    const { result } = renderHook(() => useUsage());

    await act(() => emit(USAGE_EVENT, KNOWN));

    await waitFor(() => expect(result.current).toEqual(KNOWN));
  });

  it("keeps an emitted usage over a slower load read", async () => {
    let resolveLoad: (usage: ProviderUsage[]) => void = () => undefined;
    mockIPC(
      (command) =>
        command === USAGE_COMMAND
          ? new Promise<ProviderUsage[]>((resolve) => {
              resolveLoad = resolve;
            })
          : undefined,
      { shouldMockEvents: true },
    );
    const { result } = renderHook(() => useUsage());

    await act(() => emit(USAGE_EVENT, KNOWN));
    await act(async () => {
      resolveLoad([{ provider: "codex", status: "unknown" }]);
      await Promise.resolve();
    });

    expect(result.current).toEqual(KNOWN);
  });
});
