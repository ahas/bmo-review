import { eventsAddress } from "./review";

import type { ServiceEvent } from "../../shared/types";

// The review service's one event stream (docs/api.md, 「Events」), heard by the screen through the console's
// relay: a review opened or changed moves the list, and a comment moves its review. One stream stands for the
// whole page however many parts listen, opened when the first one mounts and closed when the last one goes.

export type ReviewEventKind = "review.created" | "review.changed" | "comment.created" | "filing.created";

// One event, or `gone` where the stream could not resume and every part reads its lists again.
export type ReviewEvent = { kind: ReviewEventKind; reviewId: number | null } | { kind: "gone"; reviewId: null };

const KINDS: readonly ReviewEventKind[] = ["review.created", "review.changed", "comment.created", "filing.created"];

// How long a run of events is gathered before a part reads again, so a comment's two events are one read.
const GATHER_MS = 150;
// How long a stream that ended waits before it is opened again.
const REOPEN_MS = 3_000;

const listeners = new Set<(event: ReviewEvent) => void>();
let source: EventSource | null = null;
let reopening: ReturnType<typeof setTimeout> | null = null;

// The review an event belongs to: a review's own id, or the review its comment or filing stands under.
function reviewOf(event: ServiceEvent): number | null {
  const data = (event.data ?? {}) as { id?: unknown; reviewId?: unknown };
  const key = event.kind.startsWith("review.") ? data.id : data.reviewId;

  return typeof key === "number" ? key : null;
}

function tell(event: ReviewEvent): void {
  for (const listener of listeners) {
    listener(event);
  }
}

function open(): void {
  // A fresh stream replays the window the service keeps; what happened before it opened is what the parts read
  // when they mounted, so only what happens from now on is news.
  const openedAt = Date.now();

  source = new EventSource(eventsAddress());

  for (const kind of KINDS) {
    source.addEventListener(kind, (message) => {
      const event = JSON.parse((message as MessageEvent<string>).data) as ServiceEvent;

      if (event.at >= openedAt) {
        tell({ kind, reviewId: reviewOf(event) });
      }
    });
  }

  // A stream the browser gave up on, a resume from before the window among the reasons, is opened afresh, and
  // every part reads again what it may have missed.
  source.addEventListener("error", () => {
    if (source?.readyState !== EventSource.CLOSED) {
      return;
    }

    source.close();
    source = null;
    reopening = setTimeout(() => {
      reopening = null;

      if (listeners.size > 0) {
        open();
        tell({ kind: "gone", reviewId: null });
      }
    }, REOPEN_MS);
  });
}

// Hears every event while the calling part stands.
export function useReviewEvents(onEvent: (event: ReviewEvent) => void): void {
  if (import.meta.server) {
    return;
  }

  onMounted(() => {
    listeners.add(onEvent);

    if (source === null && reopening === null) {
      open();
    }
  });

  onScopeDispose(() => {
    listeners.delete(onEvent);

    if (listeners.size === 0) {
      source?.close();
      source = null;

      if (reopening !== null) {
        clearTimeout(reopening);
        reopening = null;
      }
    }
  });
}

export interface ReviewRefreshOptions {
  // The events that move what the part shows; `gone` moves every part.
  on: readonly ReviewEventKind[];
  // Which of them are this part's, where it shows one review rather than all.
  where?: (event: ReviewEvent) => boolean;
}

// Reads again once the events that move the part arrive, a run of them gathered into one read.
export function useReviewRefresh(run: () => void, options: ReviewRefreshOptions): void {
  let waiting: ReturnType<typeof setTimeout> | null = null;

  useReviewEvents((event) => {
    const moves = event.kind === "gone" || (options.on.includes(event.kind) && (options.where === undefined || options.where(event)));

    if (moves && waiting === null) {
      waiting = setTimeout(() => {
        waiting = null;
        run();
      }, GATHER_MS);
    }
  });

  onScopeDispose(() => {
    if (waiting !== null) {
      clearTimeout(waiting);
    }
  });
}
