// The review service's own event stream (docs/api.md, 「Events」), heard by the screen: a review made or an
// item filed moves the list, and a turn of a review's thread moves the thread. One stream stands for the whole
// page however many parts listen, opened when the first one mounts and closed when the last one goes.

export type ReviewEventType = "code-review" | "code-review-talk";

// One event: what moved and on which review. A talk's event carries `detail` once its turn ended — null where
// it was answered, the failure's code where it was not — and none while the turn is going out.
export interface ReviewEvent {
  type: ReviewEventType;
  review: number;
  detail?: string | null;
}

const TYPES: readonly ReviewEventType[] = ["code-review", "code-review-talk"];

// How long a run of events is gathered before a part reads again, so one turn's two events are one read.
const GATHER_MS = 150;

const listeners = new Set<(event: ReviewEvent) => void>();
let source: EventSource | null = null;

function open(): void {
  const base = useRuntimeConfig().app.baseURL.replace(/\/$/, "");

  source = new EventSource(`${base}/api/code/events`);

  for (const type of TYPES) {
    source.addEventListener(type, (message) => {
      const data = JSON.parse((message as MessageEvent<string>).data) as Omit<ReviewEvent, "type">;
      const event: ReviewEvent = { type, ...data };

      for (const listener of listeners) {
        listener(event);
      }
    });
  }
}

// Hears every event while the calling part stands.
export function useReviewEvents(onEvent: (event: ReviewEvent) => void): void {
  if (import.meta.server) {
    return;
  }

  onMounted(() => {
    listeners.add(onEvent);

    if (source === null) {
      open();
    }
  });

  onScopeDispose(() => {
    listeners.delete(onEvent);

    if (listeners.size === 0) {
      source?.close();
      source = null;
    }
  });
}

export interface ReviewRefreshOptions {
  // The events that move what the part shows.
  on: readonly ReviewEventType[];
  // Which of them are this part's, where it shows one review rather than all.
  where?: (event: ReviewEvent) => boolean;
}

// Reads again once the events that move the part arrive, a run of them gathered into one read.
export function useReviewRefresh(run: () => void, options: ReviewRefreshOptions): void {
  let waiting: ReturnType<typeof setTimeout> | null = null;

  useReviewEvents((event) => {
    if (!options.on.includes(event.type) || (options.where !== undefined && !options.where(event))) {
      return;
    }

    if (waiting === null) {
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
