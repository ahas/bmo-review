<template>
  <!-- The code part: the open files as tabs with how the file is read at their right end, one
       toolbar under them with the commit and what can be done to the file, and the file itself in
       the read-only editor, with the reviews of the commit shown beside and between its lines. -->
  <div class="flex min-h-0 flex-col">
    <TabBar
      v-if="props.tabs.length > 0"
      :label="$t('code.aria.open_files')"
      :tabs="barTabs"
      :active="props.path"
      @update:active="emit('tab', $event)"
      @close="emit('close-tabs', $event)"
      @move="(from: number, to: number) => emit('move-tab', from, to)"
      @pin="(id: string, on: boolean) => emit('pin-tab', id, on)"
    >
      <template v-if="readable" #switch>
        <TabSwitch v-model="showing" :items="SIDES" :label="$t('common.label.view')" :compact="props.narrow" />
      </template>
    </TabBar>

    <!-- The editor's toolbar: the explorer's side, which commit, what of the file is not shown, and
         what can be done to the file, with the reviews' side last. -->
    <ToolBar :label="$t('code.label.file')">
      <span class="!px-1">
        <UTooltip :text="props.explorerTool.label">
          <UButton
            color="neutral"
            :variant="props.explorerTool.pressed ? 'soft' : 'ghost'"
            size="sm"
            square
            :icon="props.explorerTool.icon"
            :aria-label="props.explorerTool.label"
            :aria-pressed="props.explorerTool.pressed"
            @click="props.explorerTool.onSelect()"
          />
        </UTooltip>
      </span>

      <span v-if="props.path !== null" class="min-w-0 shrink! !px-1">
        <USelectMenu
          :model-value="props.commit ?? undefined"
          :items="commitItems"
          value-key="value"
          size="sm"
          color="neutral"
          variant="ghost"
          icon="i-material-symbols-commit"
          :loading="commitsLoading"
          :search-input="{ placeholder: $t('code.placeholder.filter_commits') }"
          :filter-fields="['label', 'description']"
          :aria-label="$t('code.aria.choose_commit')"
          class="max-w-72 min-w-0"
          :content="{ align: 'start' }"
          :ui="{ content: 'max-w-[min(48rem,calc(100vw-1rem))]' }"
          @update:open="(open: boolean) => open && loadCommits()"
          @update:model-value="(value: unknown) => typeof value === 'string' && emit('commit', value === props.head ? null : value)"
        >
          <template #default>
            <span class="flex min-w-0 items-baseline gap-2">
              <span class="shrink-0 font-mono">{{ shortSha(props.commit ?? "") }}</span>
              <span v-if="shownCommit !== null && !props.narrow" class="min-w-0 truncate">{{ shownCommit.subject }}</span>
            </span>
            <UBadge v-if="props.commit === props.head" size="sm" color="neutral" variant="solid" :label="$t('code.label.current')" class="shrink-0" />
          </template>
          <template #empty>
            <span class="text-sm text-muted">{{ commitsFailure || $t("code.status.no_commits_listed") }}</span>
          </template>
        </USelectMenu>
      </span>

      <span v-if="source?.cut === true && !reading" class="text-warning" role="status">{{ $t("common.status.head_only") }}</span>

      <span class="ms-auto !px-1">
        <ActionGroup :actions="actions" :properties="properties" :properties-title="props.path" :label="$t('code.aria.file_menu')" />
      </span>

      <span v-if="!props.narrow" class="!px-1">
        <UTooltip :text="props.reviewsTool.label">
          <UButton
            color="neutral"
            :variant="props.reviewsTool.pressed ? 'soft' : 'ghost'"
            size="sm"
            square
            :icon="props.reviewsTool.icon"
            :aria-label="props.reviewsTool.label"
            :aria-pressed="props.reviewsTool.pressed"
            @click="props.reviewsTool.onSelect()"
          />
        </UTooltip>
      </span>
    </ToolBar>

    <div v-if="props.path === null" class="flex flex-col items-center gap-3 p-6 text-center">
      <p class="text-sm text-muted">{{ $t("code.message.choose_file") }}</p>
      <UButton v-if="props.narrow" size="sm" color="neutral" variant="subtle" :icon="props.explorerTool.icon" :label="$t('code.action.explorer')" @click="props.explorerTool.onSelect()" />
    </div>

    <template v-else>
      <div v-if="props.head !== null && props.commit !== props.head" class="flex shrink-0 items-center gap-2 border-b border-default bg-elevated px-3 py-1 text-sm" role="status">
        <UIcon name="i-material-symbols-history" class="size-4 shrink-0 text-toned" />
        <i18n-t :keypath="'code.message.viewing_past_commit'" tag="span" scope="global" class="min-w-0 flex-1 text-default">
          <template #commit><span class="font-mono">{{ shortSha(props.commit ?? "") }}</span></template>
        </i18n-t>
        <UButton size="sm" color="neutral" variant="subtle" :label="$t('code.action.to_current')" @click="emit('commit', null)" />
      </div>

      <ContentSkeleton v-if="reading || (source !== null && source.text !== null && !asDocument && !editorReady)" class="p-3" :label="$t('common.status.source_loading')">
        <SkeletonText :lines="10" size="sm" lastWidth="w-1/3" />
      </ContentSkeleton>
      <div v-else-if="failed !== ''" role="alert" class="flex flex-col items-start gap-2 p-3">
        <p class="text-sm text-error">{{ failed }}</p>
        <UButton size="sm" color="neutral" variant="subtle" :label="$t('common.action.refresh')" @click="retry" />
      </div>
      <p v-else-if="source !== null && source.text === null" class="p-3 text-sm text-muted">
        {{ $t("code.status.binary_file", { n: countText(source.size) }) }}
      </p>
    </template>

    <!-- The box the editor fills. Vue never puts anything in it; the editor writes into it once the
         page is the browser's own, and it is hidden rather than taken down so the editor outlives
         every file it shows.

         The box is also what the code's own menu opens on — go-to-definition for the word pressed,
         add-a-review for the lines — and the menu's trigger draws nothing of its own, so the box
         keeps its own place in the layout. The word under the press is read before the menu opens:
         a mouse says so with its own menu event, a finger with a press it goes on holding. -->
    <UContextMenu :items="codeMenu" :disabled="!standing || asDocument">
      <div
        v-show="standing && !asDocument && editorReady"
        ref="box"
        class="min-h-0 flex-1 overflow-hidden"
        :aria-label="$t('common.label.code')"
        @pointerdown.capture="notePointer"
        @contextmenu.capture="noteMenu"
      />
    </UContextMenu>

    <div
      v-show="standing && asDocument && documentReady"
      class="markdown markdown--document min-h-0 flex-1 overflow-auto p-4"
      @click="copyFromPress($t, $event, copyCode)"
      v-html="rendered"
    />

    <!-- What stands between the code lines: each review of the commit shown folded to one card,
         and the box a new line review is written in. Each is sent into a zone the editor keeps
         open for it. -->
    <template v-for="zone in zoneList" :key="zone.key">
      <Teleport v-if="zoneNodes.get(zone.key)" :to="zoneNodes.get(zone.key)">
        <div :ref="(element) => watchZone(zone.key, element as HTMLElement | null)" class="px-2 py-1 font-sans">
          <CodeReviewComposer
            v-if="zone.kind === 'composer' && chosen !== null"
            :place="choicePlace"
            :commit="shortSha(props.commit ?? '')"
            :busy="posting"
            :failure="postFailure"
            @send="postLines"
            @cancel="clearChoice"
          />
          <button
            v-else-if="zone.review !== null"
            type="button"
            class="flex w-full max-w-3xl items-center gap-2 rounded-md border px-2 py-2 text-start focus-inset"
            :class="zone.review.id === props.openReview ? 'border-accented bg-elevated' : 'border-default bg-default hover:bg-elevated'"
            @click="emit('open-review', zone.review.id)"
          >
            <UIcon name="i-material-symbols-chat-outline" class="size-3.5 shrink-0 text-toned" />
            <span class="min-w-0 flex-1 truncate text-sm text-highlighted">{{ zone.review.first }}</span>
            <span v-if="!props.narrow" class="shrink-0 text-sm text-muted tabular-nums">{{ $t("code.label.turns", { n: countText(zone.review.turns) }) }}</span>
            <span v-if="props.answering.has(zone.review.id)" class="shrink-0 text-sm text-muted">{{ $t("code.status.answering") }}</span>
            <ItemTip v-for="id in zone.review.todoIds" :id="id" :key="id" press class="shrink-0 font-mono text-sm text-muted" />
          </button>
        </div>
      </Teleport>
    </template>

    <!-- What a choice of lines turns into: beside a drag's end on a wide screen, and at the foot of
         the code on a narrow one, where the box itself opens as a sheet. -->
    <UButton
      v-if="dragged !== null && !props.narrow && chosen === null"
      size="sm"
      icon="i-material-symbols-add-comment-outline"
      :label="$t('code.action.add_review')"
      class="fixed z-20 shadow-md"
      :style="{ top: `${dragged.rect.top + dragged.rect.height + 4}px`, left: `${dragged.rect.left}px` }"
      @click="chooseDragged"
    />

    <div v-if="props.narrow && (chosen !== null || dragged !== null)" class="flex shrink-0 items-center gap-2 border-t border-default bg-elevated p-2">
      <span class="min-w-0 flex-1 truncate text-sm text-highlighted">
        <span class="font-mono">{{ narrowPlace }}</span>
      </span>
      <UButton size="sm" color="neutral" variant="ghost" :label="$t('common.action.cancel')" @click="clearChoice" />
      <UButton size="sm" icon="i-material-symbols-add-comment-outline" :label="$t('code.action.add_review')" @click="openSheet" />
    </div>

    <CodeDefinitionList v-model:open="choicesOpen" :definitions="choices" :at="choicesAt" @go="chooseDefinition" />

    <CodeReviewSheet
      v-model:open="sheetOpen"
      :scope="chosen === null ? 'line' : scopeOfLines(chosen.start, chosen.end)"
      :place="choicePlace"
      :commit="shortSha(props.commit ?? '')"
      :busy="posting"
      :failure="postFailure"
      @send="postLines"
    />
  </div>
</template>

<script setup lang="ts">
import { fileSizeText } from "../domain/files";
import { lineChoice, offeredCommits, readingPlaces, scopeOfLines, shortSha } from "../domain/code";
import { refusalText } from "../domain/refusals";
import { fileIcon } from "bmo-ui/app/domain/file-icons";
import { countText } from "bmo-ui/app/domain/numbers";
import { hasRenderedForm } from "../domain/files";
import { copyFromPress } from "bmo-ui/app/domain/code-copy";
import { fileTabLabel } from "bmo-ui/app/domain/tabs";

import type { ContextMenuItem } from "@nuxt/ui";
import type { OpenTab } from "../domain/code";
import type { CodePoint, LineSelection, ReviewView } from "../domain/review-view";
import type { BarAction, BarProperty, BarTab, SwitchItem } from "bmo-ui/app/domain/tabs";
import type { CodeCommit, CodeCommits, CodeDefinition, CodeDefinitions, CodeFile, CodeReviewSummary } from "../../shared/types";

// The code review's code part (docs/screen.md). The file is read out of the commit
// the screen stands on, never out of the working tree, and a line review is written against that
// same commit. The editor is a browser's and is fetched once the part is on the screen.

interface Props {
  project: string | null;
  tabs: OpenTab[];
  path: string | null;
  // The commit shown and the checkout's HEAD.
  commit: string | null;
  head: string | null;
  // What the commit shown and HEAD are, which the picker offers whether or not they touched the file.
  shownInfo: CodeCommit | null;
  headInfo: CodeCommit | null;
  dirty: number;
  line: number | null;
  // The worker's reviews; this part stands the ones of the file and commit shown.
  reviews: CodeReviewSummary[];
  openReview: number | null;
  // The reviews the agent is answering right now.
  answering: Set<number>;
  narrow: boolean;
  // The explorer beside the code: folded on a wide screen, a sheet from the bottom on a phone. It
  // stands first in the toolbar, on the explorer's side.
  explorerTool: BarAction;
  // The reviews beside the code, on the other side. On a phone it is one of the actions.
  reviewsTool: BarAction;
  // The screen's own tools, which stand after the file's actions.
  tools?: readonly BarAction[];
}

const props = withDefaults(defineProps<Props>(), { tools: () => [] });
const { t, locale } = useI18n();

// The host's clipboard, named here so the template can hand it on.
const copyCode = (text: string): Promise<boolean> => copyText(text);
const emit = defineEmits<{
  tab: [path: string];
  "close-tabs": [paths: string[]];
  "move-tab": [from: number, to: number];
  "pin-tab": [path: string, on: boolean];
  commit: [commit: string | null];
  "open-review": [id: number];
  created: [id: number];
  new: [place: { kind: "file" | "folder"; path: string }];
  // Where a name is declared, and the line the reader asked from — what the place they are leaving
  // keeps. Null where the address was already told, which is what happens before a list goes up.
  definition: [place: { path: string; line: number; from: number | null }];
  // The line the reader stands on, told before a list of places goes up over it. The list waits
  // for the address to carry it, since on a narrow screen the list puts a step of its own into the
  // history and a line written after that would land on the list's step instead of the reader's.
  origin: [line: number, done: () => void];
}>();

const INLINE_KEY = "bmo-code-inline-reviews";

const source = ref<CodeFile | null>(null);
const reading = ref(false);
const failed = ref("");
const editorReady = ref(false);
const documentReady = ref(false);
const rendered = ref("");
const showing = ref<"rendered" | "code">("code");
const inline = ref(true);
const commits = ref<CodeCommits | null>(null);
const commitsLoading = ref(false);
const commitsFailure = ref("");
const chosen = ref<{ start: number; end: number } | null>(null);
const dragged = ref<LineSelection | null>(null);
// Where the last press landed, and where the cursor stands. The first is what the code's menu acts
// on, the second what the file's own go-to-definition acts on, which is the route a phone has.
const pressed = ref<CodePoint | null>(null);
const cursor = ref<CodePoint | null>(null);
const seeking = ref(false);
// The places a name is declared in, when it is declared in more than one, and where on the screen
// the name that was asked about stands.
const choices = ref<CodeDefinition[]>([]);
const choicesAt = ref<{ top: number; left: number; height: number } | null>(null);
const choicesOpen = ref(false);
const sheetOpen = ref(false);
const posting = ref(false);
const postFailure = ref("");
const zoneNodes = shallowRef(new Map<string, HTMLElement>());
const box = useTemplateRef<HTMLElement>("box");
const colorMode = useColorMode();
// Where each open file was being read, so going back to a tab goes back to that place. What is
// remembered belongs to the version it was read in, which is why the commit is asked for with the
// path.
const readAt = readingPlaces();
let view: ReviewView | null = null;
let asked = 0;
let shownKey: string | null = null;

const standing = computed(() => props.path !== null && !reading.value && failed.value === "" && source.value !== null && source.value.text !== null);
const readable = computed(() => props.path !== null && hasRenderedForm(props.path));
const asDocument = computed(() => readable.value && showing.value === "rendered");

const SIDES: SwitchItem<"rendered" | "code">[] = [
  { value: "rendered", label: t("common.label.rendered"), icon: "i-material-symbols-menu-book-outline" },
  { value: "code", label: t("common.label.code"), icon: "i-material-symbols-code" }
];

const barTabs = computed<BarTab[]>(() => {
  const paths = props.tabs.map((tab) => tab.path);

  return props.tabs.map((tab) => ({ id: tab.path, label: fileTabLabel(paths, tab.path), title: tab.path, icon: fileIcon(tab.path), mono: true, pinned: tab.pinned === true }));
});

// What can be done to the file standing, at the right end of the toolbar. On a phone the reviews
// are one of them.
const actions = computed<BarAction[]>(() => {
  const path = props.path;
  const side = props.narrow ? [props.reviewsTool] : [];

  if (path === null) {
    return [...side, ...props.tools];
  }

  return [
    ...side,
    {
      id: "definition",
      label: t("code.action.go_to_definition"),
      icon: "i-material-symbols-arrow-outward",
      disabled: cursor.value?.word == null,
      loading: seeking.value,
      onSelect: () => void goToDefinition(cursor.value)
    },
    { id: "inline", label: t("code.action.show_line_reviews"), icon: "i-material-symbols-sms-outline", pressed: inline.value, onSelect: () => (inline.value = !inline.value) },
    { id: "review", label: t("code.label.review_file"), icon: "i-material-symbols-add-comment-outline", onSelect: () => emit("new", { kind: "file", path }) },
    ...props.tools
  ];
});

// The code's own menu, which is where go-to-definition stands beside add-a-review. Each item reads the
// press as it is pressed rather than as the menu was drawn, so an item is right even where the
// menu was drawn from what the press before it left.
const codeMenu = computed<ContextMenuItem[][]>(() => [
  [
    {
      label: t("code.action.go_to_definition"),
      icon: "i-material-symbols-arrow-outward",
      disabled: pressed.value?.word == null,
      onSelect: () => void goToDefinition(pressed.value)
    },
    {
      label: t("code.action.add_review"),
      icon: "i-material-symbols-add-comment-outline",
      disabled: reviewLines() === null,
      onSelect: () => reviewHere()
    }
  ]
]);

// What the file standing is, for the properties viewer.
const properties = computed<BarProperty[] | null>(() => {
  if (props.path === null) {
    return null;
  }

  const commit = shownCommit.value;

  return [
    { label: t("common.label.path"), value: props.path, mono: true },
    ...(props.project === null ? [] : [{ label: t("code.label.project"), value: props.project }]),
    { label: t("code.label.commit"), value: commit === null ? shortSha(props.commit ?? "") : `${shortSha(commit.commit)} ${commit.subject}` },
    ...(commit === null ? [] : [{ label: t("code.label.authored"), value: `${commit.author} · ${dateText.format(new Date(commit.at * 1000))}` }]),
    ...(source.value === null ? [] : [{ label: t("common.label.size"), value: fileSizeText(source.value.size, locale.value) }]),
    ...(props.dirty > 0 ? [{ label: t("code.label.uncommitted"), value: t("common.label.count", { n: countText(props.dirty) }) }] : [])
  ];
});

// The line reviews of the file and commit shown, in line order.
const lineReviews = computed(() =>
  props.reviews
    .filter((review) => review.path === props.path && review.commit === props.commit && (review.scope === "line" || review.scope === "range"))
    .sort((one, other) => (one.startLine ?? 0) - (other.startLine ?? 0) || one.id - other.id)
);

const shownCommit = computed<CodeCommit | null>(
  () => (props.shownInfo?.commit === props.commit ? props.shownInfo : null) ?? commits.value?.commits.find((one) => one.commit === props.commit) ?? null
);

const dateText = new Intl.DateTimeFormat("ko-KR", { year: "numeric", month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });

const commitItems = computed(() =>
  offeredCommits(commits.value?.commits ?? [], [props.headInfo, shownCommit.value]).map((one) => ({
    label: `${shortSha(one.commit)} ${one.subject}`,
    value: one.commit,
    description: `${one.author} · ${dateText.format(new Date(one.at * 1000))}${one.commit === props.head ? ` · ${t("code.label.current_mark")}` : ""}`
  }))
);

const choicePlace = computed(() => {
  if (chosen.value === null || props.path === null) {
    return "";
  }

  return chosen.value.start === chosen.value.end ? `${props.path}:${chosen.value.start}` : `${props.path}:${chosen.value.start}–${chosen.value.end}`;
});

const narrowPlace = computed(() => {
  const lines = chosen.value ?? dragged.value;

  if (lines === null) return "";

  return lines.start === lines.end
    ? t("common.label.line_number", { n: lines.start })
    : t("common.label.line_range", { from: lines.start, to: lines.end });
});

interface ZoneItem {
  key: string;
  afterLine: number;
  kind: "review" | "composer";
  review: CodeReviewSummary | null;
}

const zoneList = computed<ZoneItem[]>(() => {
  if (!standing.value || asDocument.value) {
    return [];
  }

  const zones: ZoneItem[] = inline.value
    ? lineReviews.value.map((review) => ({ key: `review:${review.id}`, afterLine: review.endLine ?? review.startLine ?? 1, kind: "review", review }))
    : [];

  if (chosen.value !== null && !props.narrow) {
    zones.push({ key: "composer", afterLine: chosen.value.end, kind: "composer", review: null });
  }

  return zones;
});

// Every zone's filling is measured, and the editor told the height, whenever it changes.
const observers = new Map<string, ResizeObserver>();

function watchZone(key: string, element: HTMLElement | null): void {
  observers.get(key)?.disconnect();
  observers.delete(key);

  if (element === null || typeof ResizeObserver === "undefined") {
    return;
  }

  const observer = new ResizeObserver(() => view?.resize(key, Math.ceil(element.getBoundingClientRect().height)));

  observer.observe(element);
  observers.set(key, observer);
}

function placeZones(): void {
  if (view === null) {
    return;
  }

  zoneNodes.value = view.zones(zoneList.value.map((zone) => ({ key: zone.key, afterLine: zone.afterLine })));
}

function placeMarks(): void {
  view?.mark(
    lineReviews.value.map((review) => ({ id: review.id, start: review.startLine ?? 1, end: review.endLine ?? review.startLine ?? 1 })),
    chosen.value
  );
}

function clearChoice(): void {
  chosen.value = null;
  dragged.value = null;
  postFailure.value = "";
  sheetOpen.value = false;
}

function chooseDragged(): void {
  if (dragged.value === null) {
    return;
  }

  chosen.value = { start: dragged.value.start, end: dragged.value.end };
  dragged.value = null;
  postFailure.value = "";
}

function openSheet(): void {
  chooseDragged();

  if (chosen.value !== null) {
    sheetOpen.value = true;
  }
}

// What the code's menu opens on. A mouse's right press raises the browser's own menu event; a
// finger raises nothing but a press it goes on holding, which the menu itself waits out. Both are
// read here, before the menu stands, so what it offers is the word that was actually pressed.
function notePointer(event: PointerEvent): void {
  if (event.pointerType === "mouse" && event.button !== 2) {
    return;
  }

  pressed.value = view?.pointAt(event.clientX, event.clientY) ?? null;
}

function noteMenu(event: MouseEvent): void {
  pressed.value = view?.pointAt(event.clientX, event.clientY) ?? null;
}

// What add-a-review in the menu would write on: the lines already chosen, the lines just dragged,
// or the one line the press landed on.
function reviewLines(): { start: number; end: number } | null {
  if (chosen.value !== null) {
    return chosen.value;
  }

  if (dragged.value !== null) {
    return { start: dragged.value.start, end: dragged.value.end };
  }

  return pressed.value === null ? null : { start: pressed.value.line, end: pressed.value.line };
}

function reviewHere(): void {
  const lines = reviewLines();

  if (lines === null) {
    return;
  }

  dragged.value = null;
  postFailure.value = "";
  chosen.value = lines;

  // On a wide screen the box opens among the lines themselves; on a narrow one it rises from the
  // bottom edge, as it does when the bar below the code asks for it.
  if (props.narrow) {
    sheetOpen.value = true;
  }
}

// Where a name is declared at the commit being shown. One place is gone to; several stand as a
// list the reader chooses from, at the name itself. Nothing found is nothing at all — no notice and
// no mark — and so is an answer that arrives after the reader has moved to another file, which is
// what the file and the commit it carries are for.
async function goToDefinition(point: CodePoint | null): Promise<void> {
  const { project, path, commit } = props;
  const name = point?.word ?? null;

  if (name === null || point === null || project === null || path === null || commit === null || seeking.value) {
    return;
  }

  seeking.value = true;

  try {
    const answer = await $fetch<CodeDefinitions>("/api/code/definition", { query: { project, commit, path, name } });

    if (answer.path !== props.path || answer.commit !== props.commit) {
      return;
    }

    const first = answer.definitions[0];

    if (first === undefined) {
      return;
    }

    if (answer.definitions.length === 1) {
      emit("definition", { path: first.path, line: first.line, from: point.line });

      return;
    }

    // The list stands over the place the reader is in, and on a narrow screen it puts a step of
    // its own into the history while it stands. So the address is told where they stand and the
    // list waits for it, rather than writing it afterwards onto the step that now stands between.
    await new Promise<void>((settled) => emit("origin", point.line, settled));

    choices.value = answer.definitions;
    choicesAt.value = point.rect;
    choicesOpen.value = true;
  } catch {
    // A jump that cannot be made is not news the reader has to act on.
  } finally {
    seeking.value = false;
  }
}

// A place chosen off the list. The address was told where the reader stands when the list went up,
// so the jump itself has nothing left to leave behind.
function chooseDefinition(one: CodeDefinition): void {
  choicesOpen.value = false;
  emit("definition", { path: one.path, line: one.line, from: null });
}

async function postLines(text: string): Promise<void> {
  const lines = chosen.value;

  if (lines === null || props.project === null || props.path === null || props.commit === null || posting.value) {
    return;
  }

  posting.value = true;
  postFailure.value = "";

  try {
    const answer = await $fetch<{ ok: true; review: number }>("/api/code/reviews", {
      method: "POST",
      body: {
        project: props.project,
        commit: props.commit,
        scope: scopeOfLines(lines.start, lines.end),
        path: props.path,
        startLine: lines.start,
        endLine: lines.end,
        text
      }
    });

    clearChoice();
    emit("created", answer.review);
  } catch (error) {
    postFailure.value = refusalText(t, error, t("code.error.review_post_failed"));
  } finally {
    posting.value = false;
  }
}

async function load(): Promise<void> {
  const { project, path, commit } = props;

  if (project === null || path === null || commit === null) {
    source.value = null;

    return;
  }

  const key = `${commit}:${path}`;

  if (key === shownKey && source.value !== null && failed.value === "") {
    goToLine();

    return;
  }

  if (view !== null && shownKey !== null && source.value !== null) {
    readAt.keep(source.value.commit, source.value.path, view.topLine());
  }

  const mine = ++asked;

  reading.value = true;
  failed.value = "";
  rendered.value = "";
  documentReady.value = false;
  clearChoice();
  pressed.value = null;
  cursor.value = null;
  choicesOpen.value = false;
  choices.value = [];

  try {
    const read = await $fetch<CodeFile>("/api/code/file", { query: { project, commit, path } });

    if (mine !== asked) {
      return;
    }

    source.value = read;
    shownKey = key;
  } catch (error) {
    if (mine !== asked) {
      return;
    }

    source.value = null;
    shownKey = null;
    failed.value = refusalText(t, error, t("common.error.file_read_failed"));
  }

  reading.value = false;
  await nextTick();
  showSource();
  void showDocument();
}

function showSource(): void {
  if (view === null || source.value === null || source.value.text === null) {
    return;
  }

  const kept = props.line === null ? readAt.taken(source.value.commit, source.value.path) : null;

  view.show(source.value.text, source.value.path, props.line);

  if (kept !== null) {
    view.setTopLine(kept);
  }

  placeMarks();
  placeZones();
}

function goToLine(): void {
  if (view !== null && props.line !== null) {
    view.reveal(props.line);
  }
}

async function showDocument(): Promise<void> {
  if (!import.meta.client || !asDocument.value || rendered.value !== "" || source.value?.text == null) {
    return;
  }

  const mine = asked;
  const text = source.value.text;

  try {
    const renderer = await loadMarkdownRenderer();

    if (mine !== asked) {
      return;
    }

    if (renderer === null) {
      failed.value = t("common.error.renderer_failed");

      return;
    }

    rendered.value = renderer.renderMarkdown(t, text, "document");
    documentReady.value = true;
  } catch (error) {
    if (mine === asked) {
      failed.value = refusalText(t, error, t("common.error.renderer_failed"));
    }
  }
}

async function loadCommits(): Promise<void> {
  const { project, path } = props;

  if (!import.meta.client || project === null || path === null || commitsLoading.value || commits.value?.path === path) {
    return;
  }

  commitsLoading.value = true;
  commitsFailure.value = "";

  try {
    commits.value = await $fetch<CodeCommits>("/api/code/commits", { query: { project, path } });
  } catch (error) {
    commitsFailure.value = refusalText(t, error, t("code.error.commits_read_failed"));
  } finally {
    commitsLoading.value = false;
  }
}

async function prepareEditor(): Promise<void> {
  if (!import.meta.client || view !== null) {
    return;
  }

  try {
    const { openReviewView } = await import("../domain/review-view");

    if (box.value === null) {
      return;
    }

    view = openReviewView(t, box.value, colorMode.value === "dark");

    // A press in the margin chooses that line; a second press widens the choice when it was asked
    // to — with shift on a keyboard's screen, always on a narrow one, where a finger has no shift.
    view.onLine((line, widen) => {
      dragged.value = null;
      postFailure.value = "";
      chosen.value = lineChoice(chosen.value !== null && (widen || props.narrow) ? chosen.value.start : null, line);
    });
    view.onMark((id) => emit("open-review", id));
    view.onSelection((selection) => {
      dragged.value = selection;
    });
    view.onCursor((point) => {
      cursor.value = point;
    });
    view.onDefinition((point) => void goToDefinition(point));
    editorReady.value = true;
    showSource();
  } catch (error) {
    failed.value = refusalText(t, error, t("common.error.editor_failed"));
  }
}

async function retry(): Promise<void> {
  shownKey = null;
  await load();
  await prepareEditor();
}

watch(() => [props.project, props.path, props.commit], () => void load());
watch(() => props.line, () => goToLine());
watch(
  () => [props.project, props.path],
  () => {
    commits.value = null;
    commitsFailure.value = "";
  }
);
watch([lineReviews, chosen], () => placeMarks());
watch(zoneList, () => placeZones());
watch(showing, () => void showDocument());
watch(inline, (on) => {
  try {
    localStorage.setItem(INLINE_KEY, on ? "on" : "off");
  } catch {
    // A browser that keeps nothing forgets the choice, which is all there is to lose.
  }
});
watch(
  () => colorMode.value,
  (mode) => view?.paint(mode === "dark")
);

// The commit's title stands on the chip as soon as the file does.
watch(
  () => [props.project, props.path],
  () => void loadCommits(),
  { immediate: true }
);

onMounted(async () => {
  try {
    inline.value = localStorage.getItem(INLINE_KEY) !== "off";
  } catch {
    inline.value = true;
  }

  void load();
  await prepareEditor();
});

onBeforeUnmount(() => {
  asked++;

  for (const observer of observers.values()) {
    observer.disconnect();
  }

  view?.close();
  view = null;
});
</script>
