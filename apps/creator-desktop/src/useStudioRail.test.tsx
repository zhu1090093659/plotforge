import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useStudioRail } from "./useStudioRail";

describe("useStudioRail", () => {
  it("starts both rails collapsed and toggles them independently", () => {
    const { result } = renderHook(() => useStudioRail());
    expect(result.current.railCollapsed).toBe(true);
    expect(result.current.sidebarCollapsed).toBe(true);
    act(() => result.current.toggleRail());
    expect(result.current.railCollapsed).toBe(false);
    act(() => result.current.toggleSidebar());
    expect(result.current.sidebarCollapsed).toBe(false);
    act(() => result.current.toggleRail());
    expect(result.current.railCollapsed).toBe(true);
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

  it("respects an explicit expanded rail state for deterministic call sites", () => {
    const { result } = renderHook(() => useStudioRail(false));
    expect(result.current.railCollapsed).toBe(false);
    expect(result.current.sidebarCollapsed).toBe(true);
  });
});
