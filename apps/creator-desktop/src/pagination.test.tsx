import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import {
  PaginationControls,
  usePagination,
} from "./pagination";
import { StudioI18nProvider } from "./i18n";
import type { ReactNode } from "react";

afterEach(cleanup);

function Harness({
  items,
  threshold,
  filter,
}: {
  items: string[];
  threshold?: number;
  filter?: (item: string, q: string) => boolean;
}) {
  const result = usePagination(items, { threshold, filter });
  return (
    <div>
      <PaginationControls
        needsControls={result.needsControls}
        query={result.query}
        setQuery={result.setQuery}
        page={result.page}
        setPage={result.setPage}
        totalPages={result.totalPages}
        filteredCount={result.filteredCount}
        searchAriaLabel="Search"
        searchPlaceholder="Search…"
      />
      <ul>
        {result.visible.map((item) => (
          <li key={item}>{item}</li>
        ))}
      </ul>
    </div>
  );
}

function renderHarness(
  items: string[],
  opts: { threshold?: number; filter?: (item: string, q: string) => boolean } = {},
) {
  return render(
    <StudioI18nProvider>
      <Harness items={items} threshold={opts.threshold} filter={opts.filter} />
    </StudioI18nProvider>,
  );
}

describe("usePagination / PaginationControls", () => {
  it("renders no controls and all items for a small list", () => {
    const items = ["alpha", "beta"];
    renderHarness(items, { threshold: 8 });
    expect(screen.queryByLabelText("Search")).toBeNull();
    expect(screen.queryByLabelText("Prev")).toBeNull();
    expect(screen.getByText("alpha")).toBeTruthy();
    expect(screen.getByText("beta")).toBeTruthy();
  });

  it("activates search + paging and shows only the first page for a large list", () => {
    const items = Array.from({ length: 20 }, (_, i) => `item-${i + 1}`);
    renderHarness(items, { threshold: 8 });
    expect(screen.getByLabelText("Search")).toBeTruthy();
    // First page: items 1-8 visible; item-9 not on this page.
    expect(screen.getByText("item-1")).toBeTruthy();
    expect(screen.getByText("item-8")).toBeTruthy();
    expect(screen.queryByText("item-9")).toBeNull();
    // Page indicator + prev disabled / next enabled.
    expect(screen.getByText("Page 1 of 3")).toBeTruthy();
    expect(screen.getByLabelText("Prev").hasAttribute("disabled")).toBe(true);
    expect(screen.getByLabelText("Next").hasAttribute("disabled")).toBe(false);
  });

  it("advances pages when Next is clicked", () => {
    const items = Array.from({ length: 20 }, (_, i) => `item-${i + 1}`);
    renderHarness(items, { threshold: 8 });
    fireEvent.click(screen.getByLabelText("Next"));
    expect(screen.getByText("Page 2 of 3")).toBeTruthy();
    expect(screen.queryByText("item-8")).toBeNull();
    expect(screen.getByText("item-9")).toBeTruthy();
    expect(screen.getByText("item-16")).toBeTruthy();
    expect(screen.queryByText("item-17")).toBeNull();
  });

  it("filters items by query and resets to page 1", () => {
    const items = Array.from({ length: 20 }, (_, i) => `item-${i + 1}`);
    renderHarness(items, {
      threshold: 8,
      filter: (item, q) => item.includes(q),
    });
    // Move to page 2, then search — should reset to page 1 of filtered results.
    fireEvent.click(screen.getByLabelText("Next"));
    expect(screen.getByText("Page 2 of 3")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Search"), {
      target: { value: "item-1" },
    });
    // Matches: item-1, item-10..item-19 => 11 matches => 2 pages, back on page 1.
    expect(screen.getByText("Page 1 of 2")).toBeTruthy();
    expect(screen.getByText("item-1")).toBeTruthy();
    expect(screen.queryByText("item-2")).toBeNull();
  });

  it("keeps original-index mapping stable via the visible slice", () => {
    // The hook returns the slice itself; views pair items with their original
    // index before paginating. This test guards that the visible slice is a
    // contiguous window of the source array (so paired indices stay correct).
    const items = Array.from({ length: 10 }, (_, i) => i);
    const captures: number[][] = [];
    function Probe() {
      const result = usePagination(items, { threshold: 4 });
      captures.push(result.visible.slice());
      return <PaginationControls
        needsControls={result.needsControls}
        query={result.query}
        setQuery={result.setQuery}
        page={result.page}
        setPage={result.setPage}
        totalPages={result.totalPages}
        filteredCount={result.filteredCount}
        searchAriaLabel="Search"
        searchPlaceholder="Search…"
      />;
    }
    render(
      <StudioI18nProvider>
        <Probe />
      </StudioI18nProvider>,
    );
    // First render: window [0,4).
    expect(captures.at(-1)).toEqual([0, 1, 2, 3]);
    fireEvent.click(screen.getByLabelText("Next"));
    // After paging: window [4,8).
    expect(captures.at(-1)).toEqual([4, 5, 6, 7]);
  });

  it("resets to page 1 when the underlying item list grows", () => {
    // Covers the real-world flow where a creator adds a character/rule while
    // paginated past page 1: the list identity changes and the hook should
    // pull the user back to page 1 rather than leaving them past the end.
    const initial = Array.from({ length: 20 }, (_, i) => `item-${i + 1}`);
    const { rerender } = renderHarness(initial, { threshold: 8 });
    fireEvent.click(screen.getByLabelText("Next"));
    expect(screen.getByText("Page 2 of 3")).toBeTruthy();

    // Grow the list (e.g. a character was added) — should reset to page 1.
    const grown = [
      ...initial,
      ...Array.from({ length: 8 }, (_, i) => `item-${21 + i}`),
    ];
    rerender(
      <StudioI18nProvider>
        <Harness items={grown} threshold={8} />
      </StudioI18nProvider>,
    );
    expect(screen.getByText("Page 1 of 4")).toBeTruthy();
    expect(screen.getByText("item-1")).toBeTruthy();
    expect(screen.queryByText("item-9")).toBeNull();
  });

  it("resets to page 1 when the underlying item list shrinks", () => {
    const initial = Array.from({ length: 20 }, (_, i) => `item-${i + 1}`);
    const { rerender } = renderHarness(initial, { threshold: 8 });
    fireEvent.click(screen.getByLabelText("Next"));
    fireEvent.click(screen.getByLabelText("Next"));
    expect(screen.getByText("Page 3 of 3")).toBeTruthy();

    // Shrink to 10 items (would only span 2 pages) — must not stay on page 3.
    const shrunk = initial.slice(0, 10);
    rerender(
      <StudioI18nProvider>
        <Harness items={shrunk} threshold={8} />
      </StudioI18nProvider>,
    );
    expect(screen.getByText("Page 1 of 2")).toBeTruthy();
    expect(screen.getByText("item-1")).toBeTruthy();
  });
});
