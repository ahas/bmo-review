<template>
  <!-- The explorer: which project, a search over the revision shown, and that revision's tree. Under
       the tree stands the folder chosen and what can be done with it, since a row of the tree is a
       button of its own and cannot hold another. -->
  <div class="flex min-h-0 flex-col gap-2 pt-3">
    <ChoiceSelect
      :model-value="props.project ?? undefined"
      :items="projectItems"
      value-key="value"
      size="sm"
      icon="i-material-symbols-folder-data-outline"
      :placeholder="$t('code.aria.choose_project')"
      :aria-label="$t('code.aria.choose_project')"
      class="mx-3 w-auto"
      @update:model-value="(value: unknown) => typeof value === 'string' && emit('project', value)"
    />

    <p v-if="current !== null && current.state !== 'ready'" class="px-3 text-sm text-muted" role="status">
      {{ current.name }} · {{ stateText(current.state) }}
    </p>

    <template v-else>
      <div class="flex items-center gap-2 px-3">
        <UInput
          v-model="query"
          size="sm"
          icon="i-material-symbols-search"
          :loading="searching"
          :placeholder="mode === 'name' ? $t('code.placeholder.find_file') : $t('code.placeholder.find_content')"
          :aria-label="mode === 'name' ? $t('code.placeholder.find_file') : $t('code.placeholder.find_content')"
          class="min-w-0 flex-1"
          :maxlength="CODE_VIEW.searchQueryMax"
          :ui="{ trailing: 'pe-1' }"
        >
          <template v-if="query !== ''" #trailing>
            <UTooltip :text="$t('code.shared.action.clear_search')">
              <UButton size="sm" color="neutral" variant="link" icon="i-material-symbols-close" :aria-label="$t('code.shared.action.clear_search')" @click="query = ''" />
            </UTooltip>
          </template>
        </UInput>

        <ChoiceSelect v-model="mode" :items="MODES" value-key="value" size="sm" class="shrink-0" :aria-label="$t('code.shared.aria.search_scope')" />
      </div>

      <!-- The place the tree will stand waits on both readings that fill it: the projects' until one stands chosen, then the
           tree's until its entries land (docs/screen.md, 「Loading」). -->
      <ContentSkeleton v-if="reading" class="px-3" :rows="10" listClass="flex flex-col" :label="$t('code.status.files_loading')">
        <template #default="{ index }">
        <div class="flex h-8 items-center gap-2 px-2">
            <SkeletonBar class="size-4 shrink-0 rounded-sm" />
            <SkeletonBar class="h-4" :class="index % 3 === 0 ? 'w-2/3' : index % 3 === 1 ? 'w-1/2' : 'w-3/5'" />
          </div>
        </template>
      </ContentSkeleton>
      <div v-else-if="props.projectsFailure !== ''" role="alert" class="flex flex-wrap items-center gap-2 px-3">
        <p class="text-sm text-error">{{ props.projectsFailure }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" :label="$t('code.shared.action.refresh')" :loading="props.projectsLoading" @click="emit('retry-projects')" />
      </div>
      <div v-else-if="props.treeFailure !== ''" role="alert" class="flex flex-wrap items-center gap-2 px-3">
        <p class="text-sm text-error">{{ props.treeFailure }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" :label="$t('code.shared.action.refresh')" :loading="props.treeLoading" @click="emit('retry')" />
      </div>
      <p v-else-if="props.projects.length === 0" class="px-3 text-sm text-muted">{{ $t("code.status.no_projects") }}</p>

      <template v-else-if="props.tree !== null">
        <p v-if="props.tree.cut" class="px-3 text-sm text-warning" role="status">
          {{ $t("code.status.first_only", { n: countText(props.tree.entries.length) }) }}
        </p>

        <!-- The lines a content search found, each opening its file at that line. -->
        <template v-if="mode === 'content' && query.trim() !== ''">
          <p v-if="searchFailure !== ''" class="px-3 text-sm text-error" role="alert">{{ searchFailure }}</p>
          <p v-else-if="found !== null" class="px-3 text-sm tabular-nums text-muted" role="status">
            <template v-if="found.items.length === 0">{{ $t("code.status.no_lines_found") }}</template>
            <template v-else>
              {{ $t("code.status.content_hits", { files: countText(foundGroups.length), lines: countText(found.items.length) }) }}<template v-if="found.more"> · {{ $t("code.status.partial") }}</template>
            </template>
          </p>

          <ul class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto overscroll-contain px-1 pb-3" :aria-label="$t('code.aria.content_hits')">
            <li v-for="group in foundGroups" :key="group.path" class="flex flex-col gap-1">
              <p class="truncate px-2 font-mono text-sm text-highlighted">{{ group.path }}</p>
              <button
                v-for="match in group.matches"
                :key="match.line"
                type="button"
                class="flex min-h-9 w-full items-baseline gap-2 rounded-sm px-2 py-1 text-start transition-colors hover:bg-elevated active:bg-accented focus-inset"
                @click="emit('open', { path: match.path, line: match.line })"
              >
                <span class="w-10 shrink-0 text-end font-mono text-sm text-muted tabular-nums">{{ match.line }}</span>
                <span class="min-w-0 truncate font-mono text-sm text-default">{{ match.text }}</span>
              </button>
            </li>
          </ul>
        </template>

        <template v-else>
          <p v-if="named !== null" class="px-3 text-sm tabular-nums text-muted" role="status">
            <template v-if="named.length === 0">{{ $t("code.status.no_files_found") }}</template>
            <template v-else>{{ $t("code.status.files_found", { n: countText(named.length) }) }}</template>
          </p>

          <UTree
            v-model:expanded="expanded"
            :items="items"
            :get-key="(item) => item.value"
            size="sm"
            color="primary"
            :model-value="selectedItem"
            class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-1 pb-3"
            :ui="{ linkLabel: 'min-w-0 flex-1 truncate' }"
            :aria-label="$t('code.aria.file_tree')"
            @select="onSelect"
          >
            <template #item-leading="{ item, expanded: open }">
              <UIcon
                :name="(item as CodeTreeItem).entry.kind === 'folder' && (item as CodeTreeItem).entry.path !== '' ? folderIcon((item as CodeTreeItem).entry.path, open) : (item as CodeTreeItem).icon!"
                class="size-4 shrink-0"
              />
            </template>

            <template #item-label="{ item }">
              <span class="flex min-w-0 items-center gap-2">
                <span class="truncate" :class="(item as CodeTreeItem).entry.kind === 'file' ? '' : 'font-medium'" :title="(item as CodeTreeItem).entry.path">{{ (item as CodeTreeItem).label }}</span>
                <span v-if="(item as CodeTreeItem).entry.kind === 'submodule'" class="shrink-0 text-sm text-muted">{{ $t("code.label.submodule", { revision: shortSha((item as CodeTreeItem).entry.pinnedRevision ?? "") }) }}</span>
                <span v-else-if="(item as CodeTreeItem).entry.kind === 'link'" class="shrink-0 text-sm text-muted">{{ $t("code.shared.label.link") }}</span>
              </span>
            </template>

            <template #item-trailing="{ item, expanded: open }">
              <UBadge
                v-if="(item as CodeTreeItem).comments > 0"
                size="sm"
                color="neutral"
                variant="subtle"
                icon="i-material-symbols-chat-outline"
                :label="String((item as CodeTreeItem).comments)"
                :aria-label="$t('code.aria.comment_count', { n: (item as CodeTreeItem).comments })"
              />
              <UIcon
                v-if="(item as CodeTreeItem).children?.length"
                name="i-material-symbols-keyboard-arrow-down"
                class="size-4 shrink-0 text-muted transition-transform"
                :class="open ? 'rotate-180' : ''"
              />
            </template>
          </UTree>
        </template>

        <div v-if="props.folder !== null" class="flex items-center gap-2 border-t border-default p-3">
          <UIcon name="i-material-symbols-folder-outline" class="size-4 shrink-0 text-muted" />
          <span class="min-w-0 flex-1 truncate font-mono text-sm text-highlighted">{{ props.folder === "" ? $t("code.label.whole_repository") : props.folder }}</span>
          <!-- An action on the chosen folder, not the screen's create button: its icon alone, named by its tooltip. -->
          <IconButton size="sm" icon="i-material-symbols-add-comment-outline" :label="$t('code.label.comment_folder')" @click="emit('comment-folder', props.folder)" />
        </div>
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { CODE_VIEW } from "../../shared/config";
import { codeTree, foldersAbove, ROOT_VALUE, searchNames, shortSha } from "../domain/code";
import { folderIcon } from "bmo-ui/app/domain/file-icons";
import { countText } from "bmo-ui/app/domain/numbers";
import { refusalText } from "../domain/refusals";
import { findMatches } from "../domain/review";

import type { CodeTreeItem } from "../domain/code";
import type { Match, Project, ProjectState, RevisionTree } from "../../shared/types";

// The code review's explorer (docs/screen.md). The tree is made of the revision's entries the screen already
// holds, and a name search runs over them in the browser; a content search asks the service, which searches the
// revision itself.

interface Props {
  projects: Project[];
  // How the reading of those projects stands: it fills the choice above and holds the place below until it ends. Once it has
  // answered, its place keeps what it answered, its nothing or its reason, while it is read again.
  projectsLoading: boolean;
  projectsRead: boolean;
  projectsFailure: string;
  project: string | null;
  tree: RevisionTree | null;
  treeLoading: boolean;
  // Why the revision's tree could not be read, where it could not.
  treeFailure: string;
  // How many comments at the revision shown stand on each row.
  counts: Map<string, number>;
  // The file open and the folder chosen.
  file: string | null;
  folder: string | null;
}

const { t } = useI18n();
const props = defineProps<Props>();
const emit = defineEmits<{
  project: [name: string];
  open: [target: { path: string; line: number | null }];
  folder: [path: string];
  "comment-folder": [path: string];
  retry: [];
  "retry-projects": [];
}>();

const NAMES_SHOWN = 200;

const MODES = [
  { value: "name", label: t("code.shared.label.name") },
  { value: "content", label: t("code.label.search_content") }
];

const query = ref("");
const mode = ref<"name" | "content">("name");
const found = ref<{ items: Match[]; more: boolean } | null>(null);
const searching = ref(false);
const searchFailure = ref("");
const expanded = ref<string[]>([ROOT_VALUE]);
let asked = 0;
let waiting: ReturnType<typeof setTimeout> | null = null;

const current = computed(() => props.projects.find((one) => one.name === props.project) ?? null);

// What the place under the choice waits on: the projects' reading until it has answered, then the tree's own until its entries
// land. Where a reading has answered, its place keeps what it answered, its nothing or its reason, and reads again on a spinner
// rather than taking its content back down under a fresh display.
const reading = computed(
  () => props.tree === null && props.treeFailure === "" && props.projectsFailure === "" && (!props.projectsRead || props.projects.length > 0)
);

function stateText(state: ProjectState): string {
  switch (state) {
    case "missing":
      return t("code.status.no_path");
    case "not-git":
      return t("code.status.not_git");
    case "no-commits":
      return t("code.status.no_commits");
    default:
      return "";
  }
}

const projectItems = computed(() =>
  props.projects.map((one) => ({
    label: one.name,
    value: one.name,
    description: one.state === "ready" ? (one.branch ?? t("code.label.no_branch")) : stateText(one.state),
    icon: one.state === "ready" ? "i-material-symbols-folder-data-outline" : "i-material-symbols-folder-off-outline"
  }))
);

const named = computed(() =>
  mode.value !== "name" || query.value.trim() === "" || props.tree === null ? null : searchNames(props.tree.entries, query.value, NAMES_SHOWN)
);

const items = computed(() =>
  props.tree === null
    ? []
    : codeTree(props.tree.entries, props.counts, named.value === null ? null : new Set(named.value.map((entry) => entry.path)), current.value?.name ?? t("code.label.repository"))
);

const foundGroups = computed(() => {
  const groups = new Map<string, Match[]>();

  for (const match of found.value?.items ?? []) {
    groups.set(match.path, [...(groups.get(match.path) ?? []), match]);
  }

  return [...groups].map(([path, matches]) => ({ path, matches }));
});

const selectedValue = computed(() => (props.file !== null ? `file:${props.file}` : props.folder !== null ? `folder:${props.folder}` : null));

function findItem(list: readonly CodeTreeItem[], value: string | null): CodeTreeItem | undefined {
  for (const item of list) {
    if (item.value === value) {
      return item;
    }

    const below = item.children === undefined ? undefined : findItem(item.children, value);

    if (below !== undefined) {
      return below;
    }
  }

  return undefined;
}

const selectedItem = computed(() => findItem(items.value, selectedValue.value));

function onSelect(event: { preventDefault: () => void }, item: CodeTreeItem): void {
  // What is marked open follows the address, not the press.
  if (item.entry.kind === "file") {
    event.preventDefault();
    emit("open", { path: item.entry.path, line: null });
  } else if (item.entry.kind === "folder") {
    emit("folder", item.entry.path);
  } else {
    event.preventDefault();
  }
}

async function searchContent(words: string): Promise<void> {
  const project = props.project;
  const revision = props.tree?.revision;

  if (project === null || revision === undefined) {
    return;
  }

  const mine = ++asked;

  searching.value = true;
  searchFailure.value = "";

  try {
    const answer = await findMatches(project, { q: words, revision });

    if (mine === asked) {
      found.value = answer;
    }
  } catch (error) {
    if (mine === asked) {
      found.value = null;
      searchFailure.value = refusalText(t, error, t("code.error.search_failed"));
    }
  } finally {
    if (mine === asked) {
      searching.value = false;
    }
  }
}

// A content search reads the whole revision on the service, so it waits for typing to stop.
watch([query, mode, () => props.tree?.revision], ([words, how]) => {
  if (waiting !== null) {
    clearTimeout(waiting);
  }

  asked++;
  searching.value = false;
  searchFailure.value = "";
  found.value = null;

  if (how !== "content" || words.trim() === "") {
    return;
  }

  waiting = setTimeout(() => void searchContent(words), 350);
});

// A name search unfolds everything it kept; clearing it goes back to what leads to the open file.
watch(named, (kept) => {
  expanded.value =
    kept === null
      ? [...new Set([ROOT_VALUE, ...(props.file === null ? [] : foldersAbove(props.file)), ...(props.folder ? [`folder:${props.folder}`] : [])])]
      : [ROOT_VALUE, ...kept.flatMap((entry) => foldersAbove(entry.path))];
});

// A file opened from anywhere (the address, a search, a comment) is unfolded to in the tree.
watch(
  () => props.file,
  (file) => {
    if (file !== null && named.value === null) {
      expanded.value = [...new Set([...expanded.value, ...foldersAbove(file)])];
    }
  },
  { immediate: true }
);

watch(
  () => props.project,
  () => {
    query.value = "";
    expanded.value = [ROOT_VALUE];
  }
);

onBeforeUnmount(() => {
  if (waiting !== null) {
    clearTimeout(waiting);
  }
});

</script>
