import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { OVERLAY_MODE_EVENT, useOverlayMode } from "./useOverlayMode";

describe("useOverlayMode", () => {
  beforeEach(() => {
    mockIPC(() => undefined, { shouldMockEvents: true });
  });

  it("starts click-through", () => {
    const { result } = renderHook(() => useOverlayMode());

    expect(result.current).toBe("click-through");
  });

  it("follows the mode emitted by the native side", async () => {
    const { result } = renderHook(() => useOverlayMode());

    await act(() => emit(OVERLAY_MODE_EVENT, "interactive"));
    await waitFor(() => expect(result.current).toBe("interactive"));

    await act(() => emit(OVERLAY_MODE_EVENT, "click-through"));
    await waitFor(() => expect(result.current).toBe("click-through"));
  });
});
