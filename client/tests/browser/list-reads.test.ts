import { afterAll, beforeAll, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import ko from "../../i18n/ko.json";
import { stoodUp } from "./host";
import { FILE, PROJECT, PROJECTS } from "./stand-in";

import type { Browser, Locator, Page, Route } from "playwright-core";
import type { Host } from "./host";

// How the code review screen ends the readings of the lists it holds (`docs/screen.md`, 「Loading」). A reader who sees no
// project and a reader whose reading refused are two endings, and neither may leave the loading display standing: the first
// stands its one empty line, the second its reason with the read-again button, and that button's press reads the list again and
// stands what answers in the place the display held. The display is for a place that has never answered, so a read asked again
// over an answer that came back nothing keeps that line standing with no display, its button spinning until the new answer
// lands, and once every reading of the screen has ended no display stands anywhere on it. The lists are answered from the page
// as the service answers a member whose account owns no repository, or whose agency does not answer; every other call the
// screen makes goes to the stand-in. Where a reading has to be seen while it runs, its answer is held open for that long. All
// of it runs over the dev host relaying to it, in the bundle's own Korean pack. Skipped where the browser build is not on the machine.

const ready = existsSync(chromium.executablePath());

const HOST_MS = 900_000;
const TEST_MS = 180_000;
// How long the screen is given to follow a read's end or a press.
const FOLLOWS_MS = 60_000;
// How long the stand-in holds a read's answer open, so the screen can be read while that read waits on it.
const HELD_MS = 3_000;

const WIDE = { width: 1280, height: 800 };
// Where the screen asks for its projects and for its reviews, as the dev host relays them.
const PROJECTS_PATH = "/proxy/review/screen/projects";
const REVIEWS_PATH = "/proxy/review/screen/reviews";

let host: Host | null = null;
let browser: Browser;

beforeAll(async () => {
  if (!ready) return;

  host = await stoodUp();
  browser = await chromium.launch();

  // The first pages a dev host serves may reload themselves under the reader while it finds the packages the screen imports;
  // the screen is asked for once, until its project choice stands, before the tests read it.
  const page = await browser.newPage();

  await page.goto(`${host.address}/code`);
  await page.getByText(PROJECT, { exact: true }).first().waitFor({ timeout: HOST_MS });
  await page.close();
}, HOST_MS);

afterAll(async () => {
  await browser?.close();
  await host?.down();
}, HOST_MS);

/** The screen at a wide width with one of its list reads, asked at `path`, answered by `answer`, and nothing else held back. */
async function screenWith(path: string, answer: (route: Route) => Promise<void>): Promise<Page> {
  const context = await browser.newContext({ viewport: WIDE });
  const page = await context.newPage();

  await page.route((url) => url.pathname === path, async (route) => await answer(route));
  await page.goto(`${host?.address ?? ""}/code`);

  return page;
}

function answered(body: unknown, status = 200): Parameters<Route["fulfill"]>[0] {
  return { status, contentType: "application/json", body: JSON.stringify(body) };
}

// Whether the place that reads under `name` still stands its loading display. Each place ends with its own read, and a region
// whose answer has landed stands it while another reads on, so a test names the place its own read was.
async function stillReading(page: Page, name: string): Promise<number> {
  return page.getByRole("status", { name, exact: true }).count();
}

// Whether any loading display stands anywhere on the screen: every place's region is named by the pack's words for what it
// reads, and those forms all end on the same reading verb.
const loadingName = new RegExp(`${ko.code.status.reviews_loading.slice(ko.code.status.reviews_loading.lastIndexOf(" ") + 1)}$`);

async function displaysStanding(page: Page): Promise<number> {
  return await page.getByRole("status", { name: loadingName }).count();
}

// A list read answered as refusing until the test opens it to `then`, and to nothing before that: the caller's own retry of a
// refused call is refused the same way, so only the screen's read-again button can buy the second answer.
function refusingUntil(then: unknown): { answer: (route: Route) => Promise<void>; open: () => void } {
  let opened = false;

  return {
    answer: async (route) =>
      await route.fulfill(
        opened ? answered(then) : answered({ code: "upstream-failed", error: "the agency could not be asked whose the repositories are" }, 502)
      ),
    open: () => {
      opened = true;
    }
  };
}

/** The list read answered by `then` every time it is asked, each answer held open so the screen can be read while the read
 * waits on it; `asks` says how many times the read has been asked so far. */
function heldAnswer(then: unknown): { answer: (route: Route) => Promise<void>; asks: () => number } {
  let asked = 0;

  return {
    answer: async (route) => {
      asked++;
      await Bun.sleep(HELD_MS);
      await route.fulfill(answered(then));
    },
    asks: () => asked
  };
}

// Waits until the read has been asked that many times, bounded like every other wait on the screen.
async function askedTimes(asks: () => number, times: number): Promise<void> {
  const deadline = Date.now() + FOLLOWS_MS;

  while (asks() < times) {
    if (Date.now() > deadline) throw new Error(`the read was not asked ${times} times`);

    await Bun.sleep(100);
  }
}

async function spinStanding(page: Page): Promise<number> {
  return await page.locator("[class*=animate-spin]").count();
}

// Waits until no control on the screen spins: its work, and every read that asked for it, has ended.
async function spinGone(page: Page): Promise<void> {
  await page.locator("[class*=animate-spin]").first().waitFor({ state: "detached", timeout: FOLLOWS_MS });
}

/** The reason the screen says a list read refused, and the read-again button standing under it. */
function reason(page: Page): Locator {
  return page.locator("[role=alert]", { hasText: ko.code.error["upstream-failed"] });
}

function again(block: Locator): Locator {
  return block.getByRole("button", { name: ko.code.shared.action.refresh, exact: true });
}

test.skipIf(!ready)(
  "the loading display ends in its empty line where the projects read answers none",
  async () => {
    const page = await screenWith(PROJECTS_PATH, async (route) => await route.fulfill(answered({ items: [], more: false })));

    // The read ended answering nothing the reader sees: that is said, and the tree's place is no longer reading.
    await page.getByText(ko.code.status.no_projects, { exact: true }).waitFor({ timeout: FOLLOWS_MS });
    expect(await stillReading(page, ko.code.status.files_loading)).toBe(0);

    await page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "the loading display ends in the reason and its read-again button where the projects read fails",
  async () => {
    const page = await screenWith(PROJECTS_PATH, refusingUntil({ items: [], more: false }).answer);

    await reason(page).waitFor({ timeout: FOLLOWS_MS });
    await again(reason(page)).waitFor({ timeout: FOLLOWS_MS });
    expect(await stillReading(page, ko.code.status.files_loading)).toBe(0);

    await page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "the read-again button reads the projects again and the tree takes the place the display held",
  async () => {
    const list = refusingUntil({ items: PROJECTS, more: false });
    const page = await screenWith(PROJECTS_PATH, list.answer);

    await again(reason(page)).waitFor({ timeout: FOLLOWS_MS });
    list.open();
    await again(reason(page)).click({ timeout: FOLLOWS_MS });

    // The list answered on the second reading: the tree it makes room for stands where the display stood.
    await page.getByText(FILE, { exact: true }).first().waitFor({ timeout: FOLLOWS_MS });
    expect(await reason(page).count()).toBe(0);
    expect(await stillReading(page, ko.code.status.files_loading)).toBe(0);

    await page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "the reviews reason stands with its read-again button until the reading answers again",
  async () => {
    const list = refusingUntil({ items: [], more: false });
    const page = await screenWith(REVIEWS_PATH, list.answer);

    await again(reason(page)).waitFor({ timeout: FOLLOWS_MS });
    list.open();
    await again(reason(page)).click({ timeout: FOLLOWS_MS });

    // The reviews answered empty: its own line stands where the reason stood, and the reason is gone.
    await page.getByText(ko.code.status.no_reviews, { exact: true }).waitFor({ timeout: FOLLOWS_MS });
    expect(await reason(page).count()).toBe(0);
    expect(await stillReading(page, ko.code.status.reviews_loading)).toBe(0);

    await page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "a first read of the reviews stands the loading display while it runs and ends in its empty line",
  async () => {
    const held = heldAnswer({ items: [], more: false });
    const page = await screenWith(REVIEWS_PATH, held.answer);
    const display = page.getByRole("status", { name: ko.code.status.reviews_loading, exact: true });

    // The read is asked and its answer is held open: the place has never answered, so its display stands there.
    await askedTimes(held.asks, 1);
    await display.first().waitFor({ timeout: FOLLOWS_MS });
    expect(await stillReading(page, ko.code.status.reviews_loading)).toBe(1);
    expect(await page.getByText(ko.code.status.no_reviews, { exact: true }).count()).toBe(0);

    // The read ends answering nothing: the display ends with it and the empty line takes its place.
    await display.first().waitFor({ state: "detached", timeout: FOLLOWS_MS });
    await page.getByText(ko.code.status.no_reviews, { exact: true }).waitFor({ timeout: FOLLOWS_MS });

    await page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "a re-read over the reviews' empty answer keeps its line standing and runs only its button",
  async () => {
    const held = heldAnswer({ items: [], more: false });
    const page = await screenWith(REVIEWS_PATH, held.answer);

    // The first read ended answering nothing, and every read of the screen finished: the line stands on its own.
    await askedTimes(held.asks, 1);
    await spinGone(page);
    await page.getByText(ko.code.status.no_reviews, { exact: true }).waitFor({ timeout: FOLLOWS_MS });

    // The screen's fresh read asks for the reviews again over that answer, and its answer is held open: the
    // reading is going, its button says so, and the place keeps its line rather than going back down.
    await page.getByRole("button", { name: ko.code.shared.action.refresh, exact: true }).first().click({ timeout: FOLLOWS_MS });
    await askedTimes(held.asks, 2);
    expect(await spinStanding(page)).toBeGreaterThan(0);
    expect(await stillReading(page, ko.code.status.reviews_loading)).toBe(0);
    expect(await page.getByText(ko.code.status.no_reviews, { exact: true }).count()).toBe(1);

    // The read ends: its mark goes with it and the line stays where it stood.
    await spinGone(page);
    expect(await stillReading(page, ko.code.status.reviews_loading)).toBe(0);

    await page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "no loading display stands anywhere once every read of the screen has ended",
  async () => {
    const page = await screenWith(REVIEWS_PATH, async (route) => await route.fulfill(answered({ items: [], more: false })));

    // The tree stands its file, and no control spins: every reading the screen makes has ended.
    await page.getByText(FILE, { exact: true }).first().waitFor({ timeout: FOLLOWS_MS });
    await spinGone(page);

    expect(await displaysStanding(page)).toBe(0);
    expect(await page.getByText(ko.code.status.no_reviews, { exact: true }).count()).toBe(1);

    await page.context().close();
  },
  TEST_MS
);
