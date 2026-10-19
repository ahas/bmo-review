<template>
  <!-- The code review in three parts: the explorer, the code, and the reviews. Where there is room
       they stand side by side with the component library's boundaries and the two at the sides
       fold away; where there is not, the code stands alone and the other two rise from the bottom
       edge. -->
  <UDashboardPanel id="code" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <template #header>
      <UDashboardNavbar :title="$t('screens.label.code')" icon="i-material-symbols-rate-review-outline">
        <template #right>
          <DeployMark />
          <TodoOpenButton />
          <UsageMeters />
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <p v-if="projectsFailure !== ''" role="alert" class="p-4 text-sm text-error">{{ projectsFailure }}</p>

      <div v-if="narrow" class="flex min-h-0 flex-1 flex-col">
        <CodeViewer
          v-bind="viewerProps"
          :explorer-tool="explorerTool"
          :reviews-tool="reviewsTool"
          :tools="tools"
          class="min-h-0 flex-1"
          @tab="switchTab"
          @close-tabs="closeTabs"
          @move-tab="moveOpenTab"
          @pin-tab="pinOpenTab"
          @commit="chooseCommit"
          @open-review="openReview"
          @created="onCreated"
          @new="newReview"
          @origin="markOrigin"
          @definition="jumpToDefinition"
        />

        <!-- The explorer, up from the bottom edge on a phone: it only chooses what the code shows. -->
        <UDrawer v-model:open="explorerDrawer" :title="$t('code.action.explorer')" :ui="FULL_SHEET">
          <template #body>
            <CodeExplorer
              v-bind="explorerProps"
              class="min-h-0 flex-1"
              @project="openProject"
              @open="openFile"
              @folder="chooseFolder"
              @review-folder="newReview({ kind: 'folder', path: $event })"
              @retry="loadTree"
            />
          </template>
        </UDrawer>

        <!-- The reviews, up from the bottom edge on a phone. -->
        <UDrawer v-model:open="reviewsDrawer" :title="$t('code.title.reviews')" :ui="FULL_SHEET">
          <template #body>
            <CodeReviewPane
              v-bind="paneProps"
              class="min-h-0 flex-1"
              @open="openReview"
              @close="closeReview"
              @go="goToReview"
              @new="newReview"
              @filed="loadReviews"
            />
          </template>
        </UDrawer>
      </div>

      <USplitter
        v-else
        id="code-splitter"
        ref="sides"
        auto-save-id="bmo-code-splitter"
        :items="SIDES"
        :ui="{ handle: SPLIT_HANDLE }"
        class="min-h-0 flex-1"
        @collapse="onSideFold($event, false)"
        @expand="onSideFold($event, true)"
      >
        <template #explorer>
          <CodeExplorer
            v-bind="explorerProps"
            class="min-h-0 min-w-0 flex-1"
            @project="openProject"
            @open="openFile"
            @folder="chooseFolder"
            @review-folder="newReview({ kind: 'folder', path: $event })"
            @retry="loadTree"
          />
        </template>

        <template #code>
          <CodeViewer
            v-bind="viewerProps"
            :explorer-tool="explorerTool"
            :reviews-tool="reviewsTool"
            :tools="tools"
            class="min-h-0 min-w-0 flex-1"
            @tab="switchTab"
            @close-tabs="closeTabs"
            @move-tab="moveOpenTab"
            @pin-tab="pinOpenTab"
            @commit="chooseCommit"
            @open-review="openReview"
            @created="onCreated"
            @new="newReview"
            @origin="markOrigin"
            @definition="jumpToDefinition"
          />
        </template>

        <template #reviews>
          <CodeReviewPane
            v-bind="paneProps"
            class="min-h-0 min-w-0 flex-1"
            @open="openReview"
            @close="closeReview"
            @go="goToReview"
            @new="newReview"
            @filed="loadReviews"
          />
        </template>
      </USplitter>

      <CodeReviewSheet
        v-model:open="sheetOpen"
        :scope="sheetPlace?.kind ?? 'file'"
        :place="sheetPlace === null ? '' : sheetPlace.path === '' ? $t('code.label.whole_repository') : sheetPlace.path"
        :commit="shortSha(commit ?? '')"
        :busy="posting"
        :failure="postFailure"
        @send="postPlace"
      />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { CODE_VIEW } from "../../shared/config";
import { openTab, readKeptCodeTabs, reviewCounts, shortSha } from "../domain/code";
import { refusalText } from "../domain/refusals";
import { useReviewEvents, useReviewRefresh } from "../domain/events";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { lastSeen, mergeTabs, moveTab, pinTab } from "bmo-ui/app/domain/tabs";

import type { KeptCodeTabs, OpenTab } from "../domain/code";
import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { CodeProject, CodeReviewSummary, CodeTree } from "../../shared/types";
import type { SplitterItem } from "@nuxt/ui";

// The code review (docs/screen.md; docs/code-review.md). Which worker, which commit,
// which file and line, which folder and which review are open lives in the address, so a place can
// be handed on; which files stand as tabs, pinned or not, is kept for the account on the server, and
// how wide the parts are belongs to this browser tab.

const { t } = useI18n();
const SIDES: SplitterItem[] = [
  { slot: "explorer", minSize: 14, defaultSize: 22, collapsible: true, collapsedSize: 0 },
  { slot: "code", minSize: 35, defaultSize: 53 },
  { slot: "reviews", minSize: 16, defaultSize: 25, collapsible: true, collapsedSize: 0 }
];

const route = useRoute();
const router = useRouter();
const narrow = useNarrow(`(max-width: ${CODE_VIEW.tabsMaxPx}px)`);

const projects = ref<CodeProject[]>([]);
const projectsLoading = ref(false);
const projectsFailure = ref("");
const tree = ref<CodeTree | null>(null);
const treeLoading = ref(false);
const treeFailure = ref("");
const reviews = ref<CodeReviewSummary[]>([]);
const reviewsLoading = ref(false);
const reviewsFailure = ref("");
// Each worker's open files, kept for the account; the tabs are the ones of the worker shown.
const keptTabs = ref<KeptCodeTabs>({ projects: {} });
const answering = ref(new Set<number>());
const sheetOpen = ref(false);
const sheetPlace = ref<{ kind: "file" | "folder"; path: string } | null>(null);
const posting = ref(false);
const postFailure = ref("");
const explorerOpen = ref(true);
const reviewsOpen = ref(true);
const reviewsDrawer = ref(false);
const explorerDrawer = ref(false);

// The two side parts, each opened from its own side of the toolbar: folded on a wide screen, a
// sheet from the bottom edge on a phone.
const explorerTool = computed<BarAction>(() =>
  narrow.value
    ? { id: "explorer", label: t("code.action.explorer"), icon: "i-material-symbols-account-tree-outline", pressed: explorerDrawer.value, onSelect: () => { explorerDrawer.value = true; } }
    : {
        id: "explorer",
        label: explorerOpen.value ? t("code.action.explorer_collapse") : t("code.action.explorer_expand"),
        icon: explorerOpen.value ? "i-material-symbols-left-panel-close-outline" : "i-material-symbols-left-panel-open-outline",
        pressed: explorerOpen.value,
        onSelect: () => toggleSide(0)
      }
);

const reviewsTool = computed<BarAction>(() =>
  narrow.value
    ? { id: "reviews", label: t("code.title.reviews"), icon: "i-material-symbols-chat-outline", onSelect: () => { reviewsDrawer.value = true; } }
    : {
        id: "reviews",
        label: reviewsOpen.value ? t("code.action.reviews_collapse") : t("code.action.reviews_expand"),
        icon: reviewsOpen.value ? "i-material-symbols-right-panel-close-outline" : "i-material-symbols-right-panel-open-outline",
        pressed: reviewsOpen.value,
        onSelect: () => toggleSide(2)
      }
);

// What the screen offers beside the file's own actions: a fresh read.
const tools = computed<BarAction[]>(() => [
  { id: "reread", label: t("common.action.refresh"), icon: "i-material-symbols-refresh", loading: projectsLoading.value || treeLoading.value, onSelect: () => void reread() }
]);

const sides = useTemplateRef<{ panelsRef: Array<{ collapse: () => void; expand: () => void; isCollapsed: boolean }> }>("sides");

function queryText(value: unknown): string | null {
  return typeof value === "string" ? value : null;
}

function queryNumber(value: unknown): number | null {
  return typeof value === "string" && /^[1-9]\d*$/.test(value) ? Number(value) : null;
}

const projectName = computed(() => queryText(route.query.project) ?? null);
const project = computed(() => projects.value.find((one) => one.name === projectName.value) ?? null);
const head = computed(() => project.value?.head?.commit ?? null);
const commit = computed(() => queryText(route.query.commit) || head.value);
const path = computed(() => queryText(route.query.path) || null);
const line = computed(() => queryNumber(route.query.line));
const folder = computed(() => queryText(route.query.folder));
const openId = computed(() => queryNumber(route.query.review));

const tabs = computed<OpenTab[]>({
  get: () => (projectName.value === null ? [] : (keptTabs.value.projects[projectName.value]?.tabs ?? [])),
  set: (next) => {
    const name = projectName.value;

    if (name !== null) {
      keptTabs.value.projects[name] = { tabs: next, active: keptTabs.value.projects[name]?.active ?? null };
    }
  }
});

// What the review part is about: the folder chosen, or else the file open.
const place = computed<{ kind: "file" | "folder"; path: string } | null>(() =>
  folder.value !== null ? { kind: "folder", path: folder.value } : path.value !== null ? { kind: "file", path: path.value } : null
);

const counts = computed(() => reviewCounts(reviews.value, tree.value?.commit.commit ?? null));

const explorerProps = computed(() => ({
  projects: projects.value,
  project: projectName.value,
  tree: tree.value,
  failure: treeFailure.value,
  counts: counts.value,
  file: folder.value === null ? path.value : null,
  folder: folder.value
}));

const viewerProps = computed(() => ({
  project: project.value?.state === "ready" ? projectName.value : null,
  tabs: tabs.value,
  path: path.value,
  commit: commit.value,
  head: head.value,
  shownInfo: tree.value?.commit ?? null,
  headInfo: project.value?.head ?? null,
  dirty: project.value?.dirty ?? 0,
  line: line.value,
  reviews: reviews.value,
  openReview: openId.value,
  answering: answering.value,
  narrow: narrow.value
}));

const paneProps = computed(() => ({
  reviews: reviews.value,
  loading: reviewsLoading.value,
  failure: reviewsFailure.value,
  place: place.value,
  commit: commit.value,
  openId: openId.value,
  workerLabel: project.value?.label ?? projectName.value ?? ""
}));

// The address with some of its parts changed; a part set to null is taken out. Moving about this
// screen stands in place of where it was, since a file chosen and a commit chosen are the same
// place being read differently. A jump to where a name is declared is the one thing that is a step
// of its own, so that back brings the reader to what they were reading.
async function go(parts: Record<string, string | number | null>, how: "replace" | "push" = "replace"): Promise<void> {
  const query: Record<string, string> = {};

  for (const [key, value] of Object.entries({ ...route.query, ...parts })) {
    if (typeof value === "string" || typeof value === "number") {
      query[key] = String(value);
    }
  }

  await (how === "push" ? router.push({ query }) : router.replace({ query }));
}

async function loadProjects(): Promise<void> {
  projectsLoading.value = true;
  projectsFailure.value = "";

  try {
    projects.value = await $fetch<CodeProject[]>("/api/code/projects");

    if (!projects.value.some((one) => one.name === projectName.value)) {
      const first = projects.value.find((one) => one.state === "ready") ?? projects.value[0];

      if (first !== undefined) {
        go({ project: first.name, commit: null, path: null, line: null, folder: null, review: null });
      }
    }
  } catch (error) {
    projectsFailure.value = refusalText(t, error, t("code.error.workers_failed"));
  } finally {
    projectsLoading.value = false;
  }
}

let treeAsked = 0;

async function loadTree(): Promise<void> {
  const name = projectName.value;
  const at = commit.value;
  const mine = ++treeAsked;

  treeFailure.value = "";

  if (name === null || project.value?.state !== "ready" || at === null) {
    tree.value = null;

    return;
  }

  if (tree.value?.project === name && tree.value.commit.commit === at) {
    return;
  }

  treeLoading.value = true;

  try {
    const read = await $fetch<CodeTree>("/api/code/tree", { query: { project: name, commit: at } });

    if (mine === treeAsked) {
      tree.value = read;
    }
  } catch (error) {
    if (mine === treeAsked) {
      tree.value = null;
      treeFailure.value = refusalText(t, error, t("code.error.tree_failed"));
    }
  } finally {
    if (mine === treeAsked) {
      treeLoading.value = false;
    }
  }
}

let reviewsAsked = 0;

async function loadReviews(): Promise<void> {
  const name = projectName.value;
  const mine = ++reviewsAsked;

  if (name === null) {
    reviews.value = [];

    return;
  }

  reviewsLoading.value = true;
  reviewsFailure.value = "";

  try {
    const read = await $fetch<CodeReviewSummary[]>("/api/code/reviews", { query: { project: name } });

    if (mine === reviewsAsked) {
      reviews.value = read;
    }
  } catch (error) {
    if (mine === reviewsAsked) {
      reviewsFailure.value = refusalText(t, error, t("code.error.review_read_failed"));
    }
  } finally {
    if (mine === reviewsAsked) {
      reviewsLoading.value = false;
    }
  }
}

async function reread(): Promise<void> {
  tree.value = null;
  await loadProjects();
  await Promise.all([loadTree(), loadReviews()]);
}

function openProject(name: string): void {
  if (name === projectName.value) {
    return;
  }

  const kept = keptTabs.value.projects[name];
  const standing = kept?.tabs.find((one) => one.path === kept.active) ?? null;

  tree.value = null;
  reviews.value = [];
  go({ project: name, commit: standing?.commit ?? null, path: standing?.path ?? null, line: null, folder: null, review: null });
}

// A file opened from the tree, a search or a review: it stands as a tab, at the commit shown.
function openFile(target: { path: string; line: number | null }, how: "replace" | "push" = "replace"): void {
  if (commit.value !== null) {
    tabs.value = openTab(tabs.value, target.path, commit.value, Date.now(), CODE_VIEW.openTabsMax);
  }

  void go({ path: target.path, line: target.line, folder: null }, how);
  explorerDrawer.value = false;
}

// Where the reader stands, written into the address before a list of places goes up over it. The
// viewer waits to be told it is done, since the list's own step would otherwise stand between the
// reader's place and the line being written onto it.
async function markOrigin(line: number, done: () => void): Promise<void> {
  try {
    await go({ line });
  } finally {
    done();
  }
}

// A jump to where a name is declared. The place being left keeps the line it was asked from — so
// that back brings the reader to what they were reading and not to the head of the file — and the
// place gone to is a step of its own. The two are written in that order, and the first is waited
// for, since a second navigation begun while the first is still going cancels it.
async function jumpToDefinition(target: { path: string; line: number; from: number | null }): Promise<void> {
  if (target.from !== null) {
    await go({ line: target.from });
  }

  openFile({ path: target.path, line: target.line }, "push");
}

function chooseFolder(chosen: string): void {
  go({ folder: chosen, review: null });
}

// A tab goes back to the commit it was read at.
function switchTab(chosen: string): void {
  const held = tabs.value.find((one) => one.path === chosen);

  if (held === undefined) {
    return;
  }

  tabs.value = openTab(tabs.value, chosen, held.commit, Date.now(), CODE_VIEW.openTabsMax);
  go({ path: chosen, line: null, folder: null, commit: held.commit === head.value ? null : held.commit });
}

// Tabs closed from the bar; where the file standing is among them, the tab looked at last of those
// left stands instead.
function closeTabs(closed: string[]): void {
  const left = tabs.value.filter((one) => !closed.includes(one.path));

  tabs.value = left;

  if (path.value !== null && closed.includes(path.value)) {
    const next = lastSeen(left);

    if (next === null) {
      go({ path: null, line: null });
    } else {
      switchTab(next.path);
    }
  }
}

function moveOpenTab(from: number, to: number): void {
  tabs.value = moveTab(tabs.value, from, to, (one) => one.pinned === true);
}

function pinOpenTab(chosen: string, on: boolean): void {
  const at = tabs.value.findIndex((one) => one.path === chosen);

  if (at >= 0) {
    tabs.value = pinTab(tabs.value, at, on);
  }
}

// The file standing when the address names a worker and no file: the one kept as standing. It waits
// for the worker's head, so a file read at the head is not pinned to that commit in the address.
function standKept(): void {
  const name = projectName.value;
  const kept = name === null ? undefined : keptTabs.value.projects[name];

  if (head.value !== null && path.value === null && folder.value === null && kept?.active != null) {
    switchTab(kept.active);
  }
}

watch(head, () => standKept());

useKeptTabs("code", {
  read: () => keptTabs.value,
  restore: (value) => {
    const kept = readKeptCodeTabs(value, CODE_VIEW.openTabsMax);
    const name = projectName.value;

    if (name !== null) {
      const entry = kept.projects[name] ?? { tabs: [], active: null };

      kept.projects[name] = {
        tabs: mergeTabs(entry.tabs, tabs.value, (one, other) => one.path === other.path, CODE_VIEW.openTabsMax),
        active: path.value ?? entry.active
      };
    }

    keptTabs.value = kept;
    standKept();
  }
});

function chooseCommit(chosen: string | null): void {
  const at = chosen ?? head.value;

  if (path.value !== null && at !== null) {
    tabs.value = openTab(tabs.value, path.value, at, Date.now(), CODE_VIEW.openTabsMax);
  }

  go({ commit: chosen, line: null });
}

function openReview(id: number): void {
  go({ review: id });

  if (narrow.value) {
    reviewsDrawer.value = true;
  }
}

function closeReview(): void {
  go({ review: null });
}

// A review opened from a list: the screen moves to its commit and its place.
function goToReview(review: CodeReviewSummary): void {
  const at = review.commit === head.value ? null : review.commit;

  if (review.scope === "folder") {
    go({ commit: at, folder: review.path, review: review.id });
  } else {
    tabs.value = openTab(tabs.value, review.path, review.commit, Date.now(), CODE_VIEW.openTabsMax);
    go({ commit: at, path: review.path, line: review.startLine, folder: null, review: review.id });

    reviewsDrawer.value = false;
  }
}

function newReview(target: { kind: "file" | "folder"; path: string }): void {
  sheetPlace.value = target;
  postFailure.value = "";
  sheetOpen.value = true;
}

async function postPlace(text: string): Promise<void> {
  const target = sheetPlace.value;

  if (target === null || projectName.value === null || commit.value === null || posting.value) {
    return;
  }

  posting.value = true;
  postFailure.value = "";

  try {
    const answer = await $fetch<{ ok: true; review: number }>("/api/code/reviews", {
      method: "POST",
      body: { project: projectName.value, commit: commit.value, scope: target.kind, path: target.path, text }
    });

    sheetOpen.value = false;
    await onCreated(answer.review);
  } catch (error) {
    postFailure.value = refusalText(t, error, t("code.error.review_post_failed"));
  } finally {
    posting.value = false;
  }
}

async function onCreated(id: number): Promise<void> {
  await loadReviews();
  openReview(id);
}

function toggleSide(index: number): void {
  const panel = sides.value?.panelsRef[index];

  if (panel === undefined) {
    return;
  }

  if (panel.isCollapsed) {
    panel.expand();
  } else {
    panel.collapse();
  }
}

function onSideFold(index: unknown, open: boolean): void {
  if (index === 0) {
    explorerOpen.value = open;
  } else if (index === 2) {
    reviewsOpen.value = open;
  }
}

watch(projectName, () => {
  tree.value = null;
  void loadReviews();
  standKept();
});

// The file standing is kept as the worker's standing tab.
watch([projectName, path], ([name, opened]) => {
  const kept = name === null ? undefined : keptTabs.value.projects[name];

  if (kept !== undefined && opened !== null && kept.tabs.some((one) => one.path === opened)) {
    kept.active = opened;
  }
});
watch([projectName, commit, () => project.value?.state], () => void loadTree());

// The address may name a file no tab holds yet — a link handed on, a reload.
watch(
  [path, commit],
  ([opened, at]) => {
    if (opened !== null && at !== null && !tabs.value.some((one) => one.path === opened)) {
      tabs.value = openTab(tabs.value, opened, at, Date.now(), CODE_VIEW.openTabsMax);
    }
  },
  { immediate: true }
);

// A review made or a row filed moves the list; a turn under a review moves its count. Which reviews
// the agent is answering is read off the same news: a turn starts with no detail and ends with one.
useReviewRefresh(() => void loadReviews(), {
  on: ["code-review", "code-review-talk"]
});

useReviewEvents((event) => {
  if (event.type !== "code-review-talk") {
    return;
  }

  const next = new Set(answering.value);

  if (event.detail === undefined) {
    next.add(event.review);
  } else {
    next.delete(event.review);
  }

  answering.value = next;
});

onMounted(async () => {
  await loadProjects();
  await Promise.all([loadTree(), loadReviews()]);
});
</script>
