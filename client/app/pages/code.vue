<template>
  <!-- The code review in three parts: the explorer, the code, and the reviews. Where there is room they stand side
       by side with the component library's boundaries and the two at the sides fold away; where there is not,
       the code stands alone and the other two rise from the bottom edge. -->
  <UDashboardPanel id="code" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <!-- Under a console that offers its title bar the console draws it; alone, the screen draws its own. -->
    <template v-if="!barOffered" #header>
      <UDashboardNavbar :title="$t('code.title.screen')" icon="i-material-symbols-rate-review-outline" />
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
          @revision="chooseRevision"
          @open-review="openReview"
          @posted="onPosted"
          @new="newComment"
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
              @comment-folder="newComment({ kind: 'folder', path: $event, line: null, endLine: null })"
              @retry="loadTree"
            />
          </template>
        </UDrawer>

        <!-- The reviews, up from the bottom edge on a phone. -->
        <UDrawer v-model:open="reviewsDrawer" :title="$t('code.title.reviews')" :ui="FULL_SHEET">
          <template #body>
            <CodeReviewPane
              v-bind="paneProps"
              sheet
              class="min-h-0 flex-1"
              @open="openReview"
              @close="closeReview"
              @go="goToReview"
              @go-comment="goToComment"
              @new="newReview"
              @changed="onChanged"
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
            @comment-folder="newComment({ kind: 'folder', path: $event, line: null, endLine: null })"
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
            @revision="chooseRevision"
            @open-review="openReview"
            @posted="onPosted"
            @new="newComment"
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
            @go-comment="goToComment"
            @new="newReview"
            @changed="onChanged"
          />
        </template>
      </USplitter>

      <CodeReviewSheet
        v-model:open="sheetOpen"
        :kind="sheetPlace?.kind ?? 'whole'"
        :place="sheetPlace === null ? '' : placeText($t, sheetPlace, sheetPlace.kind)"
        :revision="shortSha(revision ?? '')"
        :reviews="sheetNew ? [] : revisionReviews"
        :target="sheetNew ? null : target"
        :busy="posting"
        :failure="postFailure"
        @send="postPlace"
      />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { inject } from "vue";
import { useI18n } from "vue-i18n";
import { CODE_VIEW } from "../../shared/config";
import { commentCounts, defaultTarget, openTab, placeText, readKeptCodeTabs, shortSha } from "../domain/code";
import { readKeptTabs, writeKeptTabs } from "../domain/kept";
import { refusalText } from "../domain/refusals";
import { listComments, listProjects, listReviews, postComment, readTree } from "../domain/review";
import { useReviewRefresh } from "../domain/events";
import { FULL_SHEET, SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { lastSeen, mergeTabs, moveTab, pinTab } from "bmo-ui/app/domain/tabs";

import type { CommentPlace, KeptCodeTabs, OpenTab } from "../domain/code";
import type { CommentDraft } from "../domain/review";
import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Comment, PlacedComment, Project, Review, RevisionTree } from "../../shared/types";
import type { SplitterItem } from "@nuxt/ui";

// The code review (docs/screen.md; docs/code-review.md). Which project, which revision, which file and line,
// which folder and which review are open lives in the address, so a place can be handed on; which files stand as
// tabs, pinned or not, is kept in this browser, and so is how wide the parts are.

const { t } = useI18n();
// Whether a console offers its title bar, under the key bmo-ui's bundle contract names.
const barOffered = inject("bmo-console-title-bar", null) !== null;
const SIDES: SplitterItem[] = [
  { slot: "explorer", minSize: 14, defaultSize: 22, collapsible: true, collapsedSize: 0 },
  { slot: "code", minSize: 35, defaultSize: 53 },
  { slot: "reviews", minSize: 16, defaultSize: 25, collapsible: true, collapsedSize: 0 }
];

const route = useRoute();
const router = useRouter();
const narrow = useNarrow(`(max-width: ${CODE_VIEW.tabsMaxPx}px)`);

const projects = ref<Project[]>([]);
const projectsLoading = ref(false);
const projectsFailure = ref("");
const tree = ref<RevisionTree | null>(null);
const treeLoading = ref(false);
const treeFailure = ref("");
const reviews = ref<Review[]>([]);
const reviewsLoading = ref(false);
const reviewsFailure = ref("");
// The comments of the reviews of the revision shown, each beside its review.
const placed = ref<PlacedComment[]>([]);
// Each project's open files, kept in this browser; the tabs are the ones of the project shown.
const keptTabs = ref<KeptCodeTabs>({ projects: {} });
const sheetOpen = ref(false);
const sheetPlace = ref<CommentPlace | null>(null);
// Whether the sheet opens a new review rather than writing into one.
const sheetNew = ref(false);
const posting = ref(false);
const postFailure = ref("");
const explorerOpen = ref(true);
const reviewsOpen = ref(true);
const reviewsDrawer = ref(false);
const explorerDrawer = ref(false);

// The two side parts, each opened from its own side of the toolbar: folded on a wide screen, a sheet from the
// bottom edge on a phone.
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
  { id: "reread", label: t("code.shared.action.refresh"), icon: "i-material-symbols-refresh", loading: projectsLoading.value || treeLoading.value, onSelect: () => void reread() }
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
const head = computed(() => project.value?.head ?? null);
const revision = computed(() => queryText(route.query.revision) || head.value);
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

// The folders of the revision shown, which tell a comment on a folder from one on a file.
const folders = computed(() =>
  tree.value === null || tree.value.revision !== revision.value ? null : new Set(tree.value.entries.filter((entry) => entry.kind === "folder").map((entry) => entry.path))
);
const revisionReviews = computed(() => reviews.value.filter((one) => one.revision === revision.value));
const target = computed(() => defaultTarget(reviews.value, revision.value, openId.value));
const counts = computed(() => commentCounts(placed.value, folders.value ?? new Set()));

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
  revision: revision.value,
  head: head.value,
  uncommitted: project.value?.uncommittedCount ?? 0,
  line: line.value,
  placed: placed.value,
  reviews: revisionReviews.value,
  target: target.value,
  openReview: openId.value,
  narrow: narrow.value
}));

const paneProps = computed(() => ({
  reviews: reviews.value,
  loading: reviewsLoading.value,
  failure: reviewsFailure.value,
  revision: revision.value,
  folders: folders.value,
  openId: openId.value
}));

// The address with some of its parts changed; a part set to null is taken out. Moving about this screen stands in
// place of where it was, since a file chosen and a revision chosen are the same place being read differently. A
// jump to where a name is declared is the one thing that is a step of its own, so that back brings the reader to
// what they were reading.
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
    projects.value = await listProjects();

    if (!projects.value.some((one) => one.name === projectName.value)) {
      const first = projects.value.find((one) => one.state === "ready") ?? projects.value[0];

      if (first !== undefined) {
        go({ project: first.name, revision: null, path: null, line: null, folder: null, review: null });
      }
    }
  } catch (error) {
    projectsFailure.value = refusalText(t, error, t("code.error.projects_failed"));
  } finally {
    projectsLoading.value = false;
  }
}

let treeAsked = 0;

async function loadTree(): Promise<void> {
  const name = projectName.value;
  const at = revision.value;
  const mine = ++treeAsked;

  treeFailure.value = "";

  if (name === null || project.value?.state !== "ready" || at === null) {
    tree.value = null;

    return;
  }

  if (tree.value?.projectName === name && tree.value.revision === at) {
    return;
  }

  treeLoading.value = true;

  try {
    const read = await readTree(name, at);

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

// The project's reviews, then the comments of those of the revision shown.
async function loadReviews(): Promise<void> {
  const name = projectName.value;
  const mine = ++reviewsAsked;

  if (name === null) {
    reviews.value = [];
    placed.value = [];

    return;
  }

  reviewsLoading.value = true;
  reviewsFailure.value = "";

  try {
    const read = await listReviews(name);

    if (mine === reviewsAsked) {
      reviews.value = read;
      await loadPlaced();
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

let placedAsked = 0;

async function loadPlaced(): Promise<void> {
  const mine = ++placedAsked;
  const ofRevision = reviews.value.filter((one) => one.revision === revision.value && one.commentCount > 0);

  try {
    const read = await Promise.all(ofRevision.map(async (review) => (await listComments(review.id)).map((comment: Comment) => ({ comment, review }))));

    if (mine === placedAsked) {
      placed.value = read.flat();
    }
  } catch {
    // The comments beside the code are a view of the reviews the list already stands; a read that failed leaves
    // them as they were, and the list says what failed when it is read again.
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
  placed.value = [];
  go({ project: name, revision: standing?.revision ?? null, path: standing?.path ?? null, line: null, folder: null, review: null });
}

// A file opened from the tree, a search or a comment: it stands as a tab, at the revision shown.
function openFile(target: { path: string; line: number | null }, how: "replace" | "push" = "replace"): void {
  if (revision.value !== null) {
    tabs.value = openTab(tabs.value, target.path, revision.value, Date.now(), CODE_VIEW.openTabsMax);
  }

  void go({ path: target.path, line: target.line, folder: null }, how);
  explorerDrawer.value = false;
}

// Where the reader stands, written into the address before a list of places goes up over it. The viewer waits to
// be told it is done, since the list's own step would otherwise stand between the reader's place and the line
// being written onto it.
async function markOrigin(at: number, done: () => void): Promise<void> {
  try {
    await go({ line: at });
  } finally {
    done();
  }
}

// A jump to where a name is declared. The place being left keeps the line it was asked from, so that back brings
// the reader to what they were reading and not to the head of the file, and the place gone to is a step of its
// own. The two are written in that order, and the first is waited for, since a second navigation begun while the
// first is still going cancels it.
async function jumpToDefinition(target: { path: string; line: number; from: number | null }): Promise<void> {
  if (target.from !== null) {
    await go({ line: target.from });
  }

  openFile({ path: target.path, line: target.line }, "push");
}

function chooseFolder(chosen: string): void {
  go({ folder: chosen });
}

// A tab goes back to the revision it was read at.
function switchTab(chosen: string): void {
  const held = tabs.value.find((one) => one.path === chosen);

  if (held === undefined) {
    return;
  }

  tabs.value = openTab(tabs.value, chosen, held.revision, Date.now(), CODE_VIEW.openTabsMax);
  go({ path: chosen, line: null, folder: null, revision: held.revision === head.value ? null : held.revision });
}

// Tabs closed from the bar; where the file standing is among them, the tab looked at last of those left stands
// instead.
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

// The file standing when the address names a project and no file: the one kept as standing. It waits for the
// project's head, so a file read at the head is not pinned to that revision in the address.
function standKept(): void {
  const name = projectName.value;
  const kept = name === null ? undefined : keptTabs.value.projects[name];

  if (head.value !== null && path.value === null && folder.value === null && kept?.active != null) {
    switchTab(kept.active);
  }
}

watch(head, () => standKept());

function chooseRevision(chosen: string | null): void {
  const at = chosen ?? head.value;

  if (path.value !== null && at !== null) {
    tabs.value = openTab(tabs.value, path.value, at, Date.now(), CODE_VIEW.openTabsMax);
  }

  go({ revision: chosen, line: null });
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

// A review opened from a list: the screen moves to its revision, keeping the file open where it stands there.
function goToReview(review: Review): void {
  go({ revision: review.revision === head.value ? null : review.revision, line: null, review: review.id });
}

// A comment's place gone to: its review's revision, and the folder, or the file at its line.
function goToComment(review: Review, comment: Comment): void {
  const at = review.revision === head.value ? null : review.revision;
  const where = comment.path;

  if (where === null || where === undefined) {
    return;
  }

  if (comment.line == null && (where === "" || folders.value?.has(where) === true)) {
    go({ revision: at, folder: where, review: review.id });

    return;
  }

  tabs.value = openTab(tabs.value, where, review.revision, Date.now(), CODE_VIEW.openTabsMax);
  go({ revision: at, path: where, line: comment.line ?? null, folder: null, review: review.id });
  reviewsDrawer.value = false;
}

function newComment(place: CommentPlace): void {
  sheetPlace.value = place;
  sheetNew.value = false;
  postFailure.value = "";
  sheetOpen.value = true;
}

// A new review, opened by a comment on the whole of it.
function newReview(): void {
  sheetPlace.value = { kind: "whole", path: null, line: null, endLine: null };
  sheetNew.value = true;
  postFailure.value = "";
  sheetOpen.value = true;
}

async function postPlace(draft: CommentDraft): Promise<void> {
  const place = sheetPlace.value;

  if (place === null || projectName.value === null || revision.value === null || posting.value) {
    return;
  }

  posting.value = true;
  postFailure.value = "";

  try {
    const posted = await postComment(projectName.value, revision.value, place, draft);

    sheetOpen.value = false;
    await onPosted(posted.reviewId);
  } catch (error) {
    postFailure.value = refusalText(t, error, t("code.error.comment_post_failed"));
  } finally {
    posting.value = false;
  }
}

async function onPosted(id: number): Promise<void> {
  await loadReviews();
  openReview(id);
}

function onChanged(): void {
  void loadReviews();
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

watch(revision, () => void loadPlaced());

// The file standing is kept as the project's standing tab.
watch([projectName, path], ([name, opened]) => {
  const kept = name === null ? undefined : keptTabs.value.projects[name];

  if (kept !== undefined && opened !== null && kept.tabs.some((one) => one.path === opened)) {
    kept.active = opened;
  }
});
watch([projectName, revision, () => project.value?.state], () => void loadTree());

// The address may name a file no tab holds yet: a link handed on, a reload.
watch(
  [path, revision],
  ([opened, at]) => {
    if (opened !== null && at !== null && !tabs.value.some((one) => one.path === opened)) {
      tabs.value = openTab(tabs.value, opened, at, Date.now(), CODE_VIEW.openTabsMax);
    }
  },
  { immediate: true }
);

// The tabs are kept in this browser each time they change.
watch(keptTabs, (kept) => writeKeptTabs(kept), { deep: true });

// A review opened or changed, or a comment written, moves the list and the comments beside the code.
useReviewRefresh(() => void loadReviews(), {
  on: ["review.created", "review.changed", "comment.created"]
});

onMounted(async () => {
  const kept = readKeptCodeTabs(readKeptTabs(), CODE_VIEW.openTabsMax);
  const name = projectName.value;

  if (name !== null) {
    const entry = kept.projects[name] ?? { tabs: [], active: null };

    kept.projects[name] = {
      tabs: mergeTabs(entry.tabs, tabs.value, (one, other) => one.path === other.path, CODE_VIEW.openTabsMax),
      active: path.value ?? entry.active
    };
  }

  keptTabs.value = kept;
  await loadProjects();
  standKept();
  await Promise.all([loadTree(), loadReviews()]);
});
</script>
