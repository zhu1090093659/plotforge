import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useStudioRail } from "./useStudioRail";

const railStorageKey = "plotforge:creator-desktop:rail-collapsed";

// jsdom in this project ships a localStorage object with no item methods
// (getItem/setItem/removeItem are all undefined). The production hook guards
// on `typeof storage.getItem === "function"`, so persistence is silently
// inert here unless we install a working in-memory Storage stub. These tests
// install one to exercise the persistence contract; they restore the
// original descriptor afterward.
const originalStorage = Object.getOwnPropertyDescriptor(
  globalThis.window,
  "localStorage",
);

function installMemoryStorage(): Storage {
  const store = new Map<string, string>();
  const memoryStorage: Storage = {
    get length() {
      return store.size;
    },
    clear() {
      store.clear();
    },
    getItem(key) {
      return store.has(key) ? (store.get(key) as string) : null;
    },
    key(index) {
      return Array.from(store.keys())[index] ?? null;
    },
    removeItem(key) {
      store.delete(key);
    },
    setItem(key, value) {
      store.set(key, String(value));
    },
  };
  Object.defineProperty(globalThis.window, "localStorage", {
    configurable: true,
    get: () => memoryStorage,
  });
  return memoryStorage;
}

function restoreStorage(): void {
  if (originalStorage) {
    Object.defineProperty(globalThis.window, "localStorage", originalStorage);
  }
}

beforeEach(() => {
  installMemoryStorage();
});

afterEach(() => {
  restoreStorage();
});

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

  it("honors a stored 'true' preference when no explicit arg is given", () => {
    window.localStorage.setItem(railStorageKey, "true");
    const { result } = renderHook(() => useStudioRail());
    expect(result.current.railCollapsed).toBe(true);
  });

  it("honors a stored 'false' preference when no explicit arg is given", () => {
    window.localStorage.setItem(railStorageKey, "false");
    const { result } = renderHook(() => useStudioRail());
    expect(result.current.railCollapsed).toBe(false);
  });

  it("does not persist the auto-resolved default until the user toggles", () => {
    // jsdom has no matchMedia, so the default resolves to expanded (false).
    // The hook must NOT write that default back as if it were an explicit choice.
    const { result } = renderHook(() => useStudioRail());
    expect(result.current.railCollapsed).toBe(false);
    expect(window.localStorage.getItem(railStorageKey)).toBeNull();
    // An explicit toggle is then persisted.
    act(() => result.current.toggleRail());
    expect(window.localStorage.getItem(railStorageKey)).toBe("true");
  });
});
