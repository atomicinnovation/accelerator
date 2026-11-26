import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as fetchModule from "../../api/fetch";
import type { ConnectionState } from "../../api/reconnecting-event-source";
import { makeIndexEntry } from "../../api/test-fixtures";
import type {
  ActivityEvent,
  DocTypeKey,
  IndexEntry,
  SseEvent,
} from "../../api/types";
import type { DocEventsHandle } from "../../api/use-doc-events";
import * as docEventsModule from "../../api/use-doc-events";
import { dispatchSseEvent } from "../../api/use-doc-events";
import { renderWithRouterAt } from "../../test/router-helpers";
import { ActivityFeed } from "./ActivityFeed";
import styles from "./ActivityFeed.module.css";

// AC11 (self-caused events reach the feed) is covered at the hook layer
// in `use-doc-events.test.ts` via the `subscribe + self-caused` test —
// the ActivityFeed never consults the self-cause registry, so duplicating
// the assertion here would not add incremental coverage.

vi.mock("../../api/use-doc-events", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("../../api/use-doc-events")>();
  return {
    ...actual,
    useDocEventsContext: vi.fn(),
  };
});
vi.mock("../../api/fetch", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../api/fetch")>();
  return {
    ...actual,
    fetchActivity: vi.fn(),
    fetchDocs: vi.fn(),
  };
});

interface MountResult {
  getListener: () => ((e: SseEvent) => void) | undefined;
  queryClient: QueryClient;
}

function mockHandle(overrides: Partial<DocEventsHandle> = {}): DocEventsHandle {
  return {
    setDragInProgress: vi.fn(),
    isDragInProgress: vi.fn(() => false),
    connectionState: "open",
    justReconnected: false,
    subscribe: () => () => {},
    ...overrides,
  };
}

type Listings = Partial<
  Record<DocTypeKey, IndexEntry[] | Promise<IndexEntry[]>>
>;

function mountWith({
  initial,
  connectionState = "open" as const,
  listings = {},
}: {
  initial: ActivityEvent[] | Promise<ActivityEvent[]>;
  connectionState?: ConnectionState;
  listings?: Listings;
}): MountResult {
  let captured: ((e: SseEvent) => void) | undefined;
  vi.mocked(fetchModule.fetchActivity).mockReturnValue(
    initial instanceof Promise ? initial : Promise.resolve(initial),
  );
  vi.mocked(fetchModule.fetchDocs).mockImplementation(
    async (type) => listings[type] ?? [],
  );
  vi.mocked(docEventsModule.useDocEventsContext).mockReturnValue(
    mockHandle({
      connectionState,
      subscribe: (listener) => {
        captured = listener;
        return () => {};
      },
    }),
  );
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  renderWithRouterAt(
    <QueryClientProvider client={queryClient}>
      <ActivityFeed />
    </QueryClientProvider>,
  );
  return { getListener: () => captured, queryClient };
}

function makeEvent(overrides: Partial<ActivityEvent> = {}): ActivityEvent {
  return {
    action: "edited",
    docType: "plans",
    path: "meta/plans/foo.md",
    timestamp: "2026-05-13T12:00:00Z",
    ...overrides,
  };
}

function makeSseEvent(
  overrides: Partial<Extract<SseEvent, { type: "doc-changed" }>> = {},
): SseEvent {
  return {
    type: "doc-changed",
    action: "edited",
    docType: "plans",
    path: "meta/plans/foo.md",
    etag: "sha256-abc",
    timestamp: "2026-05-13T12:00:00Z",
    ...overrides,
  };
}

describe("ActivityFeed", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders rows from initial history with new SSE events prepended", async () => {
    const initial = [
      makeEvent({
        path: "meta/plans/old.md",
        timestamp: "2026-05-13T10:00:00Z",
      }),
    ];
    const { getListener } = mountWith({ initial });

    // Wait for initial fetch to resolve.
    await screen.findByText(/old\.md/);

    act(() => {
      getListener()!(
        makeSseEvent({
          action: "created",
          path: "meta/plans/new.md",
          timestamp: "2026-05-13T11:00:00Z",
        }),
      );
    });

    const items = screen.getAllByRole("listitem");
    expect(items[0].textContent).toMatch(/created/);
    expect(items[0].textContent).toMatch(/new\.md/);
    expect(items[1].textContent).toMatch(/edited/);
    expect(items[1].textContent).toMatch(/old\.md/);
  });

  it("shows LIVE badge when connectionState is open", async () => {
    mountWith({ initial: [makeEvent()] });
    await screen.findByText(/foo\.md/);
    expect(screen.getByTestId("activity-live-badge")).toHaveTextContent("LIVE");
  });

  it("hides LIVE badge when connectionState is closed", async () => {
    mountWith({ initial: [makeEvent()], connectionState: "closed" });
    await screen.findByText(/foo\.md/);
    expect(screen.queryByTestId("activity-live-badge")).toBeNull();
  });

  it("refreshes relative-times on each 60s tick", async () => {
    const NOW = new Date("2026-05-13T12:00:30Z").getTime();
    vi.useFakeTimers();
    vi.setSystemTime(NOW);
    try {
      const ev = makeEvent({ timestamp: "2026-05-13T12:00:30Z" });
      mountWith({ initial: [ev] });
      // Flush React Query's microtasks under fake timers.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
      expect(screen.getByText(/0s ago/)).toBeTruthy();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000);
      });
      expect(screen.getByText(/1m ago/)).toBeTruthy();

      await act(async () => {
        await vi.advanceTimersByTimeAsync(60_000);
      });
      expect(screen.getByText(/2m ago/)).toBeTruthy();
    } finally {
      vi.useRealTimers();
    }
  });

  it("fetches the initial history with limit=5", async () => {
    mountWith({ initial: [makeEvent()] });
    await screen.findByText(/foo\.md/);
    expect(fetchModule.fetchActivity).toHaveBeenCalledWith(5);
  });

  it("renders an empty state when history is empty", async () => {
    mountWith({ initial: [] });
    await screen.findByTestId("activity-empty");
    expect(screen.getByTestId("activity-empty")).toHaveTextContent(
      "No recent activity",
    );
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("shows neither empty state nor rows while loading", () => {
    // Never-resolving fetch — render synchronously and assert the
    // intermediate state has no flashing empty placeholder.
    mountWith({ initial: new Promise<ActivityEvent[]>(() => {}) });
    expect(screen.queryByTestId("activity-empty")).toBeNull();
    expect(screen.queryByRole("listitem")).toBeNull();
  });

  it("dedupes when a live event has the same identity as an initial entry", async () => {
    const a = makeEvent({
      path: "meta/plans/a.md",
      timestamp: "2026-05-13T11:00:00Z",
    });
    const b = makeEvent({
      path: "meta/plans/b.md",
      timestamp: "2026-05-13T10:00:00Z",
    });
    const { getListener } = mountWith({ initial: [a, b] });

    await screen.findByText(/a\.md/);

    // Replay event `a` via the listener — same activityRowId.
    act(() => {
      getListener()!(
        makeSseEvent({
          action: a.action,
          docType: a.docType,
          path: a.path,
          timestamp: a.timestamp,
        }),
      );
    });

    expect(screen.getAllByRole("listitem").length).toBe(2);
  });

  it("drops doc-changed events with empty action or timestamp", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const { getListener } = mountWith({ initial: [] });
    await screen.findByTestId("activity-empty");

    act(() => {
      getListener()!({
        type: "doc-changed",
        action: "" as "created",
        docType: "plans",
        path: "meta/plans/bad.md",
        timestamp: "2026-05-13T12:00:00Z",
      });
    });

    // Empty state still shown — bad event was dropped.
    expect(screen.queryByText(/bad\.md/)).toBeNull();
    expect(warn).toHaveBeenCalled();
  });

  it("orders newer initial entries above older live entries via sort step", async () => {
    // Live event arrives first with older timestamp; initial fetch
    // resolves later with a newer entry. Sort must place the newer one
    // on top.
    let resolveInitial!: (value: ActivityEvent[]) => void;
    const pending = new Promise<ActivityEvent[]>((r) => {
      resolveInitial = r;
    });
    const { getListener } = mountWith({ initial: pending });

    // Live event (older).
    await waitFor(() => expect(getListener()).toBeDefined());
    act(() => {
      getListener()!(
        makeSseEvent({
          path: "meta/plans/older-live.md",
          timestamp: "2026-05-13T10:00:00Z",
        }),
      );
    });

    // Initial fetch resolves (newer).
    await act(async () => {
      resolveInitial([
        makeEvent({
          path: "meta/plans/newer-initial.md",
          timestamp: "2026-05-13T11:00:00Z",
        }),
      ]);
    });

    await screen.findByText(/newer-initial\.md/);
    const items = screen.getAllByRole("listitem");
    expect(items[0].textContent).toMatch(/newer-initial\.md/);
    expect(items[1].textContent).toMatch(/older-live\.md/);
  });
});

function rowItems(): HTMLElement[] {
  return screen.getAllByRole("listitem");
}

function rowHref(item: HTMLElement): string | null {
  return within(item).queryByRole("link")?.getAttribute("href") ?? null;
}

function rowHrefs(): Array<string | null> {
  return rowItems().map(rowHref);
}

function setEntry(dir: string): IndexEntry {
  return makeIndexEntry({
    type: "topic-research",
    id: dir,
    slug: "shared",
    relPath: `meta/research/topics/${dir}/manifest.md`,
  });
}

function inventoryEntry(dir: string): IndexEntry {
  return makeIndexEntry({
    type: "design-inventories",
    id: dir,
    slug: "shared",
    relPath: `meta/research/design-inventories/${dir}/inventory.md`,
  });
}

function rowFor(entry: IndexEntry, timestamp: string): ActivityEvent {
  return makeEvent({ docType: entry.type, path: entry.relPath, timestamp });
}

const NEWER = "2026-05-13T11:00:00Z";
const OLDER = "2026-05-13T10:00:00Z";

describe("ActivityFeed row links", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("links each research set's row to that set's own id", async () => {
    const [a, b] = [setEntry("a"), setEntry("b")];
    mountWith({
      initial: [rowFor(a, NEWER), rowFor(b, OLDER)],
      listings: { "topic-research": [a, b] },
    });

    await waitFor(() =>
      expect(rowHrefs()).toEqual([
        "/library/topic-research/a",
        "/library/topic-research/b",
      ]),
    );
  });

  it("links each design inventory's row to that inventory's own id", async () => {
    const [a, b] = [
      inventoryEntry("2026-01-01-a"),
      inventoryEntry("2026-01-02-b"),
    ];
    mountWith({
      initial: [rowFor(a, NEWER), rowFor(b, OLDER)],
      listings: { "design-inventories": [a, b] },
    });

    await waitFor(() =>
      expect(rowHrefs()).toEqual([
        "/library/design-inventories/2026-01-01-a",
        "/library/design-inventories/2026-01-02-b",
      ]),
    );
  });

  it("finds each row's document in its own doc type's listing", async () => {
    const plan = makeIndexEntry({
      type: "plans",
      id: "plan-id",
      relPath: "meta/plans/same.md",
    });
    const workItem = makeIndexEntry({
      type: "work-items",
      id: "0042",
      relPath: "meta/work/0042-thing.md",
    });
    mountWith({
      initial: [rowFor(plan, NEWER), rowFor(workItem, OLDER)],
      listings: { plans: [plan], "work-items": [workItem] },
    });

    await waitFor(() =>
      expect(rowHrefs()).toEqual([
        "/library/plans/plan-id",
        "/library/work-items/0042",
      ]),
    );
  });

  it("links a document with no id by its filename stem", async () => {
    const plan = makeIndexEntry({ id: null, relPath: "meta/plans/foo.md" });
    mountWith({
      initial: [rowFor(plan, NEWER)],
      listings: { plans: [plan] },
    });

    await waitFor(() => expect(rowHrefs()).toEqual(["/library/plans/foo"]));
  });

  it("shows an unlinked row while its listing is pending", async () => {
    mountWith({
      initial: [makeEvent({ path: "meta/plans/foo.md" })],
      listings: { plans: new Promise<IndexEntry[]>(() => {}) },
    });

    await screen.findByText(/foo\.md/);
    expect(rowHrefs()).toEqual([null]);
  });

  it.each([
    ["research set", setEntry("deleted"), setEntry("kept")],
    [
      "plan",
      makeIndexEntry({ relPath: "meta/plans/deleted.md" }),
      makeIndexEntry({ relPath: "meta/plans/kept.md" }),
    ],
  ])("shows an unlinked row for a deleted %s", async (_, deleted, kept) => {
    mountWith({
      initial: [rowFor(deleted, NEWER), rowFor(kept, OLDER)],
      listings: { [kept.type]: [kept] },
    });

    await waitFor(() => expect(rowHref(rowItems()[1])).not.toBeNull());
    expect(rowHref(rowItems()[0])).toBeNull();
  });

  it("links a live row for a new document once its listing refetches", async () => {
    const existing = makeIndexEntry({ relPath: "meta/plans/existing.md" });
    const created = makeIndexEntry({
      id: "created-id",
      relPath: "meta/plans/created.md",
    });
    const plans = [existing];
    const { getListener, queryClient } = mountWith({
      initial: [rowFor(existing, OLDER)],
      listings: { plans },
    });
    await waitFor(() =>
      expect(rowHrefs()).toEqual(["/library/plans/existing"]),
    );

    plans.push(created);
    const event = makeSseEvent({
      action: "created",
      path: created.relPath,
      timestamp: NEWER,
    });
    act(() => {
      getListener()!(event);
      dispatchSseEvent(event, queryClient);
    });

    await waitFor(() =>
      expect(rowHrefs()).toEqual([
        "/library/plans/created-id",
        "/library/plans/existing",
      ]),
    );
  });

  it("shows an unlinked row for a template without fetching a templates listing", async () => {
    const kept = makeIndexEntry({ relPath: "meta/plans/kept.md" });
    mountWith({
      initial: [
        makeEvent({
          docType: "templates",
          path: "templates/plan.md",
          timestamp: NEWER,
        }),
        rowFor(kept, OLDER),
      ],
      listings: { plans: [kept] },
    });
    await waitFor(() => expect(rowHref(rowItems()[1])).not.toBeNull());

    expect(rowHref(rowItems()[0])).toBeNull();
    expect(fetchModule.fetchDocs).not.toHaveBeenCalledWith("templates");
  });

  it("renders an unlinked row's text without a link affordance", async () => {
    const kept = makeIndexEntry({ relPath: "meta/plans/kept.md" });
    mountWith({
      initial: [
        makeEvent({ path: "meta/plans/gone.md", timestamp: NEWER }),
        rowFor(kept, OLDER),
      ],
      listings: { plans: [kept] },
    });
    await waitFor(() => expect(rowHref(rowItems()[1])).not.toBeNull());

    const unlinked = rowItems()[0];
    expect(unlinked).toHaveTextContent(/Plans/);
    expect(unlinked).toHaveTextContent(/edited/);
    expect(unlinked).toHaveTextContent(/gone\.md/);
    expect(within(unlinked).queryByRole("link")).toBeNull();
    expect(unlinked.querySelector(`.${styles.row}`)).not.toBeNull();
    expect(unlinked.querySelector(`.${styles.linkedRow}`)).toBeNull();
    expect(rowItems()[1].querySelector(`.${styles.linkedRow}`)).not.toBeNull();
  });
});
