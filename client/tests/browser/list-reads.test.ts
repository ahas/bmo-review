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
// stands what answers in the place the display held. The lists are answered from the page as the service answers a member whose
// account owns no repository, or whose agency does not answer; every other call the screen makes goes to the stand-in. All of it
// runs over the dev host relaying to it, in the bundle's own Korean pack. Skipped where the browser build is not on the machine.

const ready = existsSync(chromium.executablePath());

const HOST_MS = 900_000;
const TEST_MS = 180_000;
// How long the screen is given to follow a read's end or a press.
const FOLLOWS_MS = 60_000;

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

/** The places of the screen that announce themselves as still reading what they show, by the name each reads under. */
async function reading(page: Page): Promise<number> {
  const files = await page.getByRole("status", { name: ko.code.status.files_loading, exact: true }).count();
  const reviews = await page.getByRole("status", { name: ko.code.status.reviews_loading, exact: true }).count();

  return files + reviews;
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

    // The read ended answering nothing the reader sees: that is said, and no place is left reading.
    await page.getByText(ko.code.status.no_projects, { exact: true }).waitFor({ timeout: FOLLOWS_MS });
    expect(await reading(page)).toBe(0);

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
    expect(await reading(page)).toBe(0);

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
    expect(await reading(page)).toBe(0);

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
    expect(await reading(page)).toBe(0);

    await page.context().close();
  },
  TEST_MS
);
