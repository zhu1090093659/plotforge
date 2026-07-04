import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useStudioI18n } from "./i18n";
import { StudioButton } from "./studioUi";

// ---------------------------------------------------------------------------
// usePagination — list pagination + search for data-driven Studio views.
//
// The Studio shell's main content area is a single scroll container. For
// collection views (Characters, Rules, State, Assets) the list of cards can
// grow unboundedly with project data; without pagination that forces a long
// vertical scroll. This hook keeps small lists exactly as they were (no
// controls rendered) and only activates search + paging once the list exceeds
// a threshold, so behavior-based tests with small fixtures are unaffected.
// ---------------------------------------------------------------------------

export interface PaginationResult<T> {
  /** Current search query string. */
  query: string;
  setQuery(value: string): void;
  /** 1-indexed current page. */
  page: number;
  setPage(value: number): void;
  /** Total page count (>=1). */
  totalPages: number;
  /** Total items after filtering. */
  filteredCount: number;
  /** The slice of items visible on the current page. */
  visible: T[];
  /** True when search + paging controls should render. */
  needsControls: boolean;
}

export function usePagination<T>(
  items: T[],
  opts: {
    /** List size above which search + paging activate. Defaults to 8. */
    threshold?: number;
    /**
     * Predicate that returns true when the item matches the (trimmed) query.
     * Only consulted when controls are active; small lists bypass filtering.
     */
    filter?: (item: T, query: string) => boolean;
  } = {},
): PaginationResult<T> {
  const threshold = opts.threshold ?? 8;
  const filter = opts.filter;
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(1);

  const needsControls = items.length > threshold;

  const filtered = useMemo(() => {
    const trimmed = query.trim();
    if (!needsControls || !trimmed || !filter) {
      return items;
    }
    return items.filter((item) => filter(item, trimmed));
  }, [items, query, filter, needsControls]);

  const totalPages = needsControls
    ? Math.max(1, Math.ceil(filtered.length / threshold))
    : 1;
  const effectivePage = Math.min(page, totalPages);

  // Reset to the first page whenever the query or the underlying item list
  // changes, so a stale page index never lands past the last page.
  useEffect(() => {
    setPage(1);
  }, [query, items.length]);

  const visible = needsControls
    ? filtered.slice(
        (effectivePage - 1) * threshold,
        effectivePage * threshold,
      )
    : filtered;

  return {
    query,
    setQuery,
    page: effectivePage,
    setPage,
    totalPages,
    filteredCount: filtered.length,
    visible,
    needsControls,
  };
}

// ---------------------------------------------------------------------------
// PaginationControls — search input + prev/next + page indicator.
//
// Renders nothing when `needsControls` is false (small lists). The page
// indicator uses an accessible live region so screen readers announce the
// current page after navigation.
// ---------------------------------------------------------------------------

export interface PaginationControlsProps {
  needsControls: boolean;
  query: string;
  setQuery(value: string): void;
  page: number;
  setPage(value: number): void;
  totalPages: number;
  filteredCount: number;
  /** Accessible label for the search input. */
  searchAriaLabel: string;
  /** Visible placeholder for the search input. */
  searchPlaceholder: string;
}

export function PaginationControls({
  needsControls,
  query,
  setQuery,
  page,
  setPage,
  totalPages,
  filteredCount,
  searchAriaLabel,
  searchPlaceholder,
}: PaginationControlsProps) {
  const { t } = useStudioI18n();
  if (!needsControls) {
    return null;
  }
  return (
    <div className="flex flex-wrap items-center gap-2">
      <input
        type="search"
        aria-label={searchAriaLabel}
        placeholder={searchPlaceholder}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        className="h-9 min-w-[12rem] flex-1 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
      />
      <div
        className="flex items-center gap-2"
        role="group"
        aria-label={t("pagination.pageGroup")}
      >
        <StudioButton
          variant="secondary"
          aria-label={t("pagination.prev")}
          onClick={() => setPage(Math.max(1, page - 1))}
          disabled={page <= 1}
        >
          {t("pagination.prev")}
        </StudioButton>
        <span
          aria-live="polite"
          className="text-xs font-semibold text-ink/70"
        >
          {t("pagination.pageOf", { page, total: totalPages })}
        </span>
        <StudioButton
          variant="secondary"
          aria-label={t("pagination.next")}
          onClick={() => setPage(Math.min(totalPages, page + 1))}
          disabled={page >= totalPages}
        >
          {t("pagination.next")}
        </StudioButton>
      </div>
      <span className="text-xs text-ink/55">
        {t("pagination.filteredCount", { count: filteredCount })}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PaginatedCardGrid — convenience wrapper that renders controls above a card
// grid. Kept as a thin layout helper so each view stays declarative.
// ---------------------------------------------------------------------------

export function PaginatedCardGrid({
  controls,
  children,
}: {
  controls: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-3">
      {controls}
      <div className="grid gap-3 lg:grid-cols-2">{children}</div>
    </div>
  );
}
