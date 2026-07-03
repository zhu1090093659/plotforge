import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useStudioRail } from "./useStudioRail";

describe("useStudioRail", () => {
  it("starts expanded by default and toggles to collapsed", () => {
    const { result } = renderHook(() => useStudioRail());
    expect(result.current.railCollapsed).toBe(false);
    act(() => result.current.toggleRail());
    expect(result.current.railCollapsed).toBe(true);
    act(() => result.current.toggleRail());
    expect(result.current.railCollapsed).toBe(false);
  });

  it("respects an explicit initial collapsed state", () => {
    const { result } = renderHook(() => useStudioRail(true));
    expect(result.current.railCollapsed).toBe(true);
  });

  it("keeps a stable toggle identity across renders", () => {
    const { result, rerender } = renderHook(() => useStudioRail());
    const first = result.current.toggleRail;
    rerender();
    expect(result.current.toggleRail).toBe(first);
  });
});
