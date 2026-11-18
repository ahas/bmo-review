import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";

import { chromium } from "playwright-core";

import ko from "../../i18n/ko.json";
import { stoodUp } from "./host";
import { FILE, PROJECT } from "./stand-in";

import type { Browser, Locator, Page } from "playwright-core";
import type { Host } from "./host";

// The box a comment is written in has one frame wherever it stands (`docs/screen.md`, 「Reviewing」). Among the code
// on a wide screen its zone draws a border that sets it apart from the code lines and the box draws none. In the
// modal, a line's on a narrow screen and a file's on a wide one, the dialog is the one frame: the box draws no
// border, and nothing between the dialog and the box draws a frame, a border round it or a shadow; the line the
// dialog draws between its body and its foot is a divider, one edge alone, and no frame. Each is read off the page
// as computed borders in both schemes. The modal asks before throwing away what was written in it and closes at once
// with nothing written, and its foot stays on a screen as short as a phone's with its keyboard up. All of it runs
// over the dev host relaying to a stand-in of the service. Skipped where the browser build is not on the machine.

const ready = existsSync(chromium.executablePath());

const HOST_MS = 900_000;
const TEST_MS = 120_000;
// How long the screen is given to follow a press, or to stand once its page is asked for.
const FOLLOWS_MS = 60_000;

const WIDE = { width: 1280, height: 800 };
const PHONE = { width: 360, height: 780 };
// A phone's screen with its keyboard up: what is left above the keyboard.
const PHONE_TYPING = { width: 360, height: 400 };
const SCREEN = `/code?project=${PROJECT}&path=${FILE}`;
// The layer's colour mode is read from this cookie, dark where none is set.
const SCHEME_COOKIE = "nuxt-color-mode";
const SCHEMES = ["dark", "light"] as const;

type Scheme = (typeof SCHEMES)[number];
type Viewport = { width: number; height: number };

/** What an element draws around itself, as the browser computed it. */
interface Drawn {
  borders: number[];
  shadow: string;
  background: string;
}

let host: Host | null = null;
let browser: Browser;

beforeAll(async () => {
  if (!ready) return;

  host = await stoodUp();
  browser = await chromium.launch();

  // The first pages a dev host serves may reload themselves under the reader while it finds the packages the
  // screen imports; the screen is asked for once, until its code stands, before the tests read it.
  const page = await browser.newPage();

  await page.goto(`${host.address}${SCREEN}`);
  await lineNumbers(page).first().waitFor({ timeout: HOST_MS });
  await page.close();
}, HOST_MS);

afterAll(async () => {
  await browser?.close();
  await host?.down();
}, HOST_MS);

function lineNumbers(page: Page): Locator {
  return page.locator(".monaco-editor .margin .line-numbers");
}

/** The screen at a width and in a scheme with the file open. */
async function opened(viewport: Viewport, scheme: Scheme): Promise<Page> {
  const context = await browser.newContext({ viewport });
  const address = host?.address ?? "";

  await context.addCookies([{ name: SCHEME_COOKIE, value: scheme, url: address }]);

  const page = await context.newPage();

  await page.goto(`${address}${SCREEN}`);
  await lineNumbers(page).nth(1).waitFor({ timeout: FOLLOWS_MS });
  // The colour mode marks the page's root with the scheme it reads in.
  await page.waitForFunction((wanted) => document.documentElement.classList.contains(wanted), scheme, { timeout: FOLLOWS_MS });

  return page;
}

/** The screen with its second line chosen by a press on its number. */
async function chosenLine(viewport: Viewport, scheme: Scheme): Promise<Page> {
  const page = await opened(viewport, scheme);

  await lineNumbers(page).nth(1).click({ timeout: FOLLOWS_MS });

  return page;
}

/** The modal a narrow screen opens a chosen line's comment in, standing with its box. */
async function lineSheet(viewport: Viewport, scheme: Scheme): Promise<{ page: Page; dialog: Locator }> {
  const page = await chosenLine(viewport, scheme);

  await page.getByRole("button", { name: ko.code.action.add_comment, exact: true }).click({ timeout: FOLLOWS_MS });

  const dialog = page.getByRole("dialog");

  await dialog.locator("form textarea").waitFor({ timeout: FOLLOWS_MS });

  return { page, dialog };
}

async function closed(page: Page, dialogs: number): Promise<number> {
  await page.waitForFunction((count) => document.querySelectorAll("[role=dialog]").length === count, dialogs, { timeout: FOLLOWS_MS });

  return page.locator("[role=dialog]").count();
}

/** What the element and each element above it up to `top`, left out, draw around themselves. */
function drawnUpTo(element: Locator, top: string | null): Promise<Drawn[]> {
  return element.evaluate((start, stop) => {
    const drawn: Drawn[] = [];

    for (let at: Element | null = start; at !== null && (stop === null ? drawn.length < 2 : !at.matches(stop)); at = at.parentElement) {
      const style = getComputedStyle(at);

      drawn.push({
        borders: [style.borderTopWidth, style.borderRightWidth, style.borderBottomWidth, style.borderLeftWidth].map((width) => Number.parseFloat(width)),
        shadow: style.boxShadow,
        background: style.backgroundColor
      });
    }

    return drawn;
  }, top);
}

const BARE = [0, 0, 0, 0];
const CLEAR = "rgba(0, 0, 0, 0)";

/** The dialog is the box's one frame: the box bare, nothing between them framed, the dialog drawing its edge. */
async function oneFrameInDialog(dialog: Locator): Promise<void> {
  const form = dialog.locator("form");

  await form.waitFor({ timeout: FOLLOWS_MS });

  const between = await drawnUpTo(form, "[role=dialog]");
  const [outer] = await drawnUpTo(dialog, null);

  expect(between.length).toBeGreaterThan(1);
  expect(between[0]?.borders).toEqual(BARE);
  expect(between.map((drawn) => drawn.borders.filter((width) => width > 0).length <= 1)).toEqual(between.map(() => true));
  expect(between.map((drawn) => drawn.shadow)).toEqual(between.map(() => "none"));
  expect(outer !== undefined && (outer.borders.some((width) => width > 0) || outer.shadow !== "none")).toBe(true);
}

describe.each(SCHEMES)("in the %s scheme", (scheme) => {
  test.skipIf(!ready)(
    "among the code on a wide screen the zone draws the box's one frame",
    async () => {
      const page = await chosenLine(WIDE, scheme);
      // The editor keeps a zone open between the lines and draws what fills it as a widget over that room.
      const form = page.locator(".monaco-editor .overlayWidgets form");

      await form.waitFor({ timeout: FOLLOWS_MS });

      const [box, frame] = await drawnUpTo(form, null);

      expect(box?.borders).toEqual(BARE);
      expect(box?.background).toBe(CLEAR);
      expect(frame?.borders.every((width) => width > 0)).toBe(true);
      expect(frame?.background).not.toBe(CLEAR);

      await page.context().close();
    },
    TEST_MS
  );

  test.skipIf(!ready)(
    "in the modal a narrow screen opens the dialog is the box's one frame",
    async () => {
      const { page, dialog } = await lineSheet(PHONE, scheme);

      await oneFrameInDialog(dialog);
      await page.context().close();
    },
    TEST_MS
  );

  test.skipIf(!ready)(
    "in the modal a wide screen opens a file's comment in the dialog is the box's one frame",
    async () => {
      const page = await opened(WIDE, scheme);
      const action = page.getByRole("button", { name: ko.code.label.comment_file, exact: true });

      // The file's actions stand as buttons where the toolbar has room for them, and in its menu otherwise.
      if ((await action.count()) > 0) {
        await action.click({ timeout: FOLLOWS_MS });
      } else {
        await page.getByRole("button", { name: ko.code.aria.file_menu, exact: true }).click({ timeout: FOLLOWS_MS });
        await page.getByRole("menuitem", { name: ko.code.label.comment_file }).click({ timeout: FOLLOWS_MS });
      }

      await oneFrameInDialog(page.getByRole("dialog"));
      await page.context().close();
    },
    TEST_MS
  );
});

test.skipIf(!ready)(
  "the modal closes at once with nothing written, and asks before throwing written words away",
  async () => {
    const untouched = await lineSheet(PHONE, "dark");

    await untouched.page.keyboard.press("Escape");
    expect(await closed(untouched.page, 0)).toBe(0);
    await untouched.page.context().close();

    const written = await lineSheet(PHONE, "dark");
    const words = written.dialog.locator("form textarea");

    await words.fill("x");
    await words.fill("");
    await written.page.keyboard.press("Escape");
    expect(await closed(written.page, 0)).toBe(0);
    await written.page.context().close();

    const kept = await lineSheet(PHONE, "dark");
    const address = kept.page.url();

    await kept.dialog.locator("form textarea").fill("x");
    await kept.page.keyboard.press("Escape");
    expect(await closed(kept.page, 2)).toBe(2);

    // Back answers the question as staying: the modal stays with its words, and the page where it was.
    await kept.page.goBack();
    expect(await closed(kept.page, 1)).toBe(1);
    expect(await kept.page.getByRole("dialog").locator("form textarea").inputValue()).toBe("x");
    expect(kept.page.url()).toBe(address);
    await kept.page.context().close();
  },
  TEST_MS
);

test.skipIf(!ready)(
  "the modal and its foot stay on a phone's screen with its keyboard up",
  async () => {
    const { page, dialog } = await lineSheet(PHONE_TYPING, "dark");
    const send = dialog.getByRole("button", { name: ko.code.shared.action.send, exact: true });
    const [frame, button] = await Promise.all([dialog.boundingBox(), send.boundingBox()]);

    expect(frame !== null && frame.y >= 0 && frame.y + frame.height <= PHONE_TYPING.height).toBe(true);
    expect(button !== null && button.y >= 0 && button.y + button.height <= PHONE_TYPING.height).toBe(true);

    await page.context().close();
  },
  TEST_MS
);
