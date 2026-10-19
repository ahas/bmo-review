<template>
  <!-- The review part: the reviews of what is open, or one review's thread. -->
  <div class="flex min-h-0 flex-col gap-2 pt-3">
    <template v-if="props.openId === null">
      <div class="flex items-center gap-2 px-3">
        <p class="min-w-0 flex-1 truncate text-sm font-medium text-highlighted">{{ $t("code.title.reviews") }}</p>
        <UButton
          v-if="props.place !== null && props.commit !== null"
          size="sm"
          color="neutral"
          variant="subtle"
          icon="i-material-symbols-add-comment-outline"
          :label="props.place.kind === 'folder' ? $t('code.label.review_folder') : $t('code.label.review_file')"
          @click="emit('new', props.place)"
        />
      </div>

      <p v-if="props.place === null" class="px-3 text-sm text-muted">{{ $t("code.message.choose_path") }}</p>
      <p v-else-if="props.failure !== ''" class="px-3 text-sm text-error" role="alert">{{ props.failure }}</p>
      <ContentSkeleton v-else-if="props.loading && props.reviews.length === 0" class="px-3" :rows="4" listClass="flex flex-col gap-1" :label="$t('code.status.reviews_loading')">
        <div class="flex h-5 items-center gap-2 py-1">
          <SkeletonBar class="h-4 w-16" />
          <SkeletonBar class="h-4 min-w-0 flex-1" />
        </div>
      </ContentSkeleton>

      <template v-else>
        <p v-if="groups.here.length === 0 && groups.elsewhere.length === 0" class="px-3 text-sm text-muted">{{ $t("code.status.no_reviews") }}</p>

        <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-3 pb-3">
          <section v-if="groups.here.length > 0" :aria-label="$t('code.aria.reviews_of_commit')">
            <p class="mb-1 text-sm text-muted">{{ $t("code.label.this_commit") }} · <span class="font-mono">{{ shortSha(props.commit ?? "") }}</span></p>
            <ul class="flex flex-col gap-1">
              <li v-for="review in groups.here" :key="review.id">
                <CodeReviewRow :review="review" @open="emit('open', review.id)" />
              </li>
            </ul>
          </section>

          <section v-for="group in groups.elsewhere" :key="group.commit" :aria-label="$t('code.aria.reviews_of', { commit: shortSha(group.commit) })">
            <p class="mb-1 text-sm text-muted">{{ $t("code.label.other_commit") }} · <span class="font-mono">{{ shortSha(group.commit) }}</span></p>
            <ul class="flex flex-col gap-1">
              <li v-for="review in group.reviews" :key="review.id">
                <CodeReviewRow :review="review" @open="emit('go', review)" />
              </li>
            </ul>
          </section>
        </div>
      </template>
    </template>

    <template v-else>
      <div class="flex items-center gap-2 px-3">
        <UTooltip :text="$t('code.action.back_to_reviews')">
          <UButton size="sm" color="neutral" variant="ghost" icon="i-material-symbols-arrow-back" :aria-label="$t('code.action.back_to_reviews')" @click="emit('close')" />
        </UTooltip>
        <button
          v-if="thread !== null"
          type="button"
          class="flex min-h-9 min-w-0 flex-1 flex-col rounded-sm px-1 text-start transition-colors hover:bg-elevated active:bg-accented focus-inset"
          :title="$t('code.action.view_in_code', { place: reviewPlace($t, thread.review) })"
          @click="emit('go', thread.review)"
        >
          <span class="truncate font-mono text-sm text-highlighted">{{ reviewPlace($t, thread.review) }}</span>
          <span class="truncate text-sm text-muted">
            {{ $t("code.label.review_head", { scope: $t(SCOPE_LABELS[thread.review.scope]), id: thread.review.id }) }} <span class="font-mono">{{ shortSha(thread.review.commit) }}</span>
          </span>
        </button>
      </div>

      <div v-if="thread !== null" class="flex flex-wrap items-center gap-1 px-3 text-sm text-muted">
        <template v-if="thread.review.todoIds.length > 0">
          {{ $t("code.label.filed_items") }}
          <ItemTip v-for="id in thread.review.todoIds" :id="id" :key="id" press>
            <UButton
              size="sm"
              color="neutral"
              variant="subtle"
              :label="`#${id}`"
              :aria-label="$t('code.aria.open_todo', { id })"
              @click="openTodo(id)"
            />
          </ItemTip>
        </template>
        <UButton
          size="sm"
          color="neutral"
          variant="subtle"
          icon="i-material-symbols-playlist-add"
          :label="$t('code.action.file_todo')"
          class="ms-auto"
          :disabled="thread.answering"
          @click="filing = true"
        />
        <p v-if="thread.answering" class="w-full text-end">{{ $t("code.status.file_after_answer") }}</p>
      </div>

      <ContentSkeleton v-if="thread === null && threadFailure === ''" class="px-3" :rows="4" listClass="flex flex-col gap-3" :label="$t('code.status.reviews_loading')">
        <div class="flex flex-col gap-1">
          <SkeletonLine width="w-16" />
          <SkeletonText :lines="2" size="sm" lastWidth="w-2/3" />
        </div>
      </ContentSkeleton>
      <div v-else-if="threadFailure !== ''" role="alert" class="flex flex-wrap items-center gap-2 px-3">
        <p class="text-sm text-error">{{ threadFailure }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" :label="$t('common.action.refresh')" @click="load" />
      </div>

      <template v-if="thread !== null">
        <div v-if="!thread.readable" class="mx-3 flex flex-col gap-1 rounded-md bg-elevated p-2">
          <p class="text-sm text-warning">{{ $t("code.status.excerpt_only") }}</p>
          <pre v-if="thread.review.excerpt !== null" class="overflow-x-auto font-mono text-sm whitespace-pre text-default">{{ thread.review.excerpt }}</pre>
        </div>

        <ol ref="list" class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-3" :aria-label="$t('code.aria.review_thread')">
          <li v-for="turn in thread.turns" :key="turn.id" class="flex flex-col gap-1">
            <p class="text-sm">
              <span class="font-medium" :class="turn.speaker === 'owner' ? 'text-highlighted' : 'text-muted'">{{ turn.speaker === "owner" ? $t("common.label.speaker_owner") : $t("common.label.speaker_bmo") }}</span>
              <span class="text-muted"> · {{ timeText(turn.saidAt) }}</span>
            </p>
            <MarkdownText :text="turn.text" class="text-sm text-highlighted" />
          </li>
        </ol>

        <p v-if="thread.answering" class="px-3 text-sm text-muted" role="status">{{ $t("common.status.bmo_answering") }}</p>
        <p v-else-if="thread.failure !== null" class="px-3 text-sm text-warning" role="status">{{ $t(`code.error.${thread.failure}`) }}</p>
        <p v-if="sendFailure !== ''" class="px-3 text-sm text-error" role="alert">{{ sendFailure }}</p>

        <form class="flex flex-col gap-2 border-t border-default p-3" @submit.prevent="send">
          <UTextarea
            v-model="text"
            :rows="2"
            size="sm"
            autoresize
            :maxrows="8"
            :maxlength="TALK_LIMIT"
            :disabled="thread.answering"
            :placeholder="$t('code.placeholder.talk')"
            :aria-label="$t('code.aria.talk')"
            @keydown.meta.enter.prevent="send"
            @keydown.ctrl.enter.prevent="send"
          />
          <UButton type="submit" size="sm" class="self-end" icon="i-material-symbols-send-outline" :label="$t('common.action.send')" :disabled="text.trim() === '' || thread.answering" :loading="sending" />
        </form>

        <CodeReviewTodo
          v-model:open="filing"
          :review="thread.review"
          :turns="thread.turns"
          :worker-label="props.workerLabel"
          :answering="thread.answering"
          @filed="onFiled"
        />
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { TALK_LIMIT } from "../../shared/config";
import { reviewGroups, reviewPlace, SCOPE_LABELS, shortSha } from "../domain/code";
import { refusalText } from "../domain/refusals";
import { useReviewRefresh } from "../domain/events";

import type { CodeReviewSummary, CodeReviewThread } from "../../shared/types";

// The review part of the code review (docs/screen.md). The list is sorted out of
// the worker's reviews the screen already holds; a thread is read on its own and kept up by the
// news that it moved, the way the TODO panel keeps an item's talk.

interface Props {
  reviews: CodeReviewSummary[];
  loading: boolean;
  failure: string;
  place: { kind: "file" | "folder"; path: string } | null;
  commit: string | null;
  openId: number | null;
  workerLabel: string;
}

const { t } = useI18n();
const props = defineProps<Props>();
const emit = defineEmits<{
  open: [id: number];
  close: [];
  go: [review: CodeReviewSummary];
  new: [place: { kind: "file" | "folder"; path: string }];
  filed: [id: number];
}>();

const route = useRoute();
const router = useRouter();

const thread = ref<CodeReviewThread | null>(null);
const threadFailure = ref("");
const text = ref("");
const sending = ref(false);
const sendFailure = ref("");
const filing = ref(false);
const list = useTemplateRef<HTMLElement>("list");
let asked = 0;

const groups = computed(() => (props.place === null ? { here: [], elsewhere: [] } : reviewGroups(props.reviews, props.place, props.commit)));

const clock = new Intl.DateTimeFormat("ko-KR", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });

function timeText(seconds: number): string {
  return clock.format(new Date(seconds * 1000));
}

async function load(): Promise<void> {
  const id = props.openId;

  if (id === null) {
    thread.value = null;

    return;
  }

  const mine = ++asked;

  threadFailure.value = "";

  try {
    const answer = await $fetch<CodeReviewThread>(`/api/code/reviews/${id}`);

    if (mine === asked) {
      thread.value = answer;
    }
  } catch (error) {
    if (mine === asked) {
      threadFailure.value = refusalText(t, error, t("code.error.review_read_failed"));
    }
  }
}

async function send(): Promise<void> {
  const said = text.value.trim();
  const id = props.openId;

  if (said === "" || id === null || sending.value || thread.value?.answering === true) {
    return;
  }

  sending.value = true;
  sendFailure.value = "";

  try {
    await $fetch(`/api/code/reviews/${id}/talk`, { method: "POST", body: { text: said } });
    text.value = "";
    await load();
  } catch (error) {
    // Words the bot refused were not written down, so they stay in the box to send again.
    sendFailure.value = refusalText(t, error, t("common.error.send_failed"));
  } finally {
    sending.value = false;
  }
}

// The address opens the item, in the drawer where the panel is one.
function openTodo(id: number): void {
  void router.replace({ query: { ...route.query, todo: String(id) } });
}

async function onFiled(id: number): Promise<void> {
  emit("filed", id);
  await load();
}

watch(
  () => props.openId,
  () => {
    thread.value = null;
    text.value = "";
    sendFailure.value = "";
    filing.value = false;
    void load();
  },
  { immediate: true }
);

// The newest turn is the one waited for, so the thread goes to its foot when it grows.
watch(
  () => thread.value?.turns.length,
  async () => {
    await nextTick();

    if (list.value !== null) {
      list.value.scrollTop = list.value.scrollHeight;
    }
  }
);

useReviewRefresh(() => void load(), {
  on: ["code-review-talk", "code-review"],
  where: (event) => (event.type === "code-review-talk" || event.type === "code-review") && event.review === props.openId
});
</script>
