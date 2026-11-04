import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { OVERLAY_MODE_COMMAND, OVERLAY_MODE_EVENT, useOverlayMode } from "./useOverlayMode";

function mockNativeMode(mode: string) {
  mockIPC((command) => (command === OVERLAY_MODE_COMMAND ? mode : undefined), {
    shouldMockEvents: true,
  });
}

describe("useOverlayMode", () => {
  it("starts click-through", () => {
    mockNativeMode("click-through");
    const { result } = renderHook(() => useOverlayMode());

    expect(result.current).toBe("click-through");
  });

  it("shows the native mode read on load", async () => {
    mockNativeMode("interactive");
    const { result } = renderHook(() => useOverlayMode());

    await waitFor(() => expect(result.current).toBe("interactive"));
  });

  it("follows the mode emitted by the native side", async () => {
    mockNativeMode("click-through");
    const { result } = renderHook(() => useOverlayMode());

    await act(() => emit(OVERLAY_MODE_EVENT, "interactive"));
    await waitFor(() => expect(result.current).toBe("interactive"));

    await act(() => emit(OVERLAY_MODE_EVENT, "click-through"));
    await waitFor(() => expect(result.current).toBe("click-through"));
  });
});
