<template>
  <!-- The review part: the project's reviews, or one review and its comments. -->
  <div class="flex min-h-0 flex-col gap-2 pt-3">
    <template v-if="props.openId === null">
      <!-- The head: the title on a row of its own, where no sheet's title names the part already, and the screen's
           create button on the row under it, at its right end. On a phone the screen floats that button over the
           code instead, so the row stands nothing there. -->
      <p v-if="!props.sheet" class="truncate px-3 text-sm font-medium text-highlighted">{{ $t("code.title.reviews") }}</p>
      <div v-if="props.revision !== null && !phone" class="flex items-center justify-end gap-2 px-3">
        <CreateButton :label="$t('code.action.open_review')" @click="emit('new')" />
      </div>

      <div v-if="props.failure !== ''" role="alert" class="flex flex-wrap items-center gap-2 px-3">
        <p class="text-sm text-error">{{ props.failure }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" :label="$t('code.shared.action.refresh')" :loading="props.loading" @click="emit('retry')" />
      </div>
      <ContentSkeleton v-else-if="props.loading && !props.reviewsRead" class="px-3" :rows="4" listClass="flex flex-col gap-1" :label="$t('code.status.reviews_loading')">
        <div class="flex h-5 items-center gap-2 py-1">
          <SkeletonBar class="h-4 w-16" />
          <SkeletonBar class="h-4 min-w-0 flex-1" />
        </div>
      </ContentSkeleton>

      <template v-else>
        <p v-if="groups.here.length === 0 && groups.elsewhere.length === 0" class="px-3 text-sm text-muted">{{ $t("code.status.no_reviews") }}</p>

        <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-3 pb-3">
          <section v-if="groups.here.length > 0" :aria-label="$t('code.aria.reviews_of_revision')">
            <p class="mb-1 text-sm text-muted">{{ $t("code.label.this_revision") }} · <span class="font-mono">{{ shortSha(props.revision ?? "") }}</span></p>
            <ul class="flex flex-col gap-1">
              <li v-for="review in groups.here" :key="review.id">
                <CodeReviewRow :review="review" @open="emit('open', review.id)" />
              </li>
            </ul>
          </section>

          <section v-for="group in groups.elsewhere" :key="group.revision" :aria-label="$t('code.aria.reviews_of', { revision: shortSha(group.revision) })">
            <p class="mb-1 text-sm text-muted">{{ $t("code.label.other_revision") }} · <span class="font-mono">{{ shortSha(group.revision) }}</span></p>
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
      <!-- The head: the review's title on a row of its own; under it the way back to the list at the left end, what
           the review is, and what can be done to it at the right end; under those the items filed from it. -->
      <p v-if="review !== null" class="px-3 text-sm font-medium break-words text-highlighted">{{ review.title }}</p>
      <div class="flex items-center gap-2 px-3">
        <UTooltip :text="$t('code.action.back_to_reviews')">
          <UButton size="sm" color="neutral" variant="ghost" square icon="i-material-symbols-arrow-back" :aria-label="$t('code.action.back_to_reviews')" @click="emit('close')" />
        </UTooltip>
        <template v-if="review !== null">
          <UBadge size="sm" :color="STATE_COLORS[review.state]" variant="subtle" :label="$t(STATE_LABELS[review.state])" class="shrink-0" />
          <span class="min-w-0 truncate text-sm text-muted">{{ review.authorName }}</span>
          <UTooltip :text="$t('code.action.view_revision', { revision: shortSha(review.revision) })">
            <UButton size="sm" color="neutral" variant="ghost" class="shrink-0 font-mono" :label="shortSha(review.revision)" @click="emit('go', review)" />
          </UTooltip>
          <span class="ms-auto flex shrink-0 items-center gap-1">
            <UTooltip :text="stateAction">
              <UButton
                size="sm"
                color="neutral"
                variant="ghost"
                square
                :icon="review.state === 'closed' ? 'i-material-symbols-lock-open-outline' : 'i-material-symbols-check-circle-outline'"
                :aria-label="stateAction"
                :loading="changing"
                @click="toggleState"
              />
            </UTooltip>
            <UTooltip :text="$t('code.action.file_item')">
              <UButton size="sm" color="neutral" variant="ghost" square icon="i-material-symbols-playlist-add" :aria-label="$t('code.action.file_item')" @click="filing = true" />
            </UTooltip>
          </span>
        </template>
      </div>
      <p v-if="review !== null && review.itemIds.length > 0" class="flex flex-wrap items-center gap-1 px-3 text-sm text-muted">
        {{ $t("code.label.filed_items") }}
        <span v-for="id in review.itemIds" :key="id" class="font-mono text-highlighted tabular-nums">#{{ id }}</span>
      </p>
      <p v-if="changeFailure !== ''" class="px-3 text-sm text-error" role="alert">{{ changeFailure }}</p>

      <ContentSkeleton v-if="review === null && readFailure === ''" class="px-3" :rows="4" listClass="flex flex-col gap-3" :label="$t('code.status.reviews_loading')">
        <div class="flex flex-col gap-1">
          <SkeletonLine width="w-16" />
          <SkeletonText :lines="2" size="sm" lastWidth="w-2/3" />
        </div>
      </ContentSkeleton>
      <div v-else-if="readFailure !== ''" role="alert" class="flex flex-wrap items-center gap-2 px-3">
        <p class="text-sm text-error">{{ readFailure }}</p>
        <UButton class="ms-auto" size="sm" color="neutral" variant="subtle" :label="$t('code.shared.action.refresh')" :loading="reviewLoading" @click="load" />
      </div>

      <template v-if="review !== null">
        <p v-if="comments.length === 0" class="px-3 text-sm text-muted">{{ $t("code.status.no_comments") }}</p>

        <ol ref="list" class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-3" :aria-label="$t('code.aria.review_comments')">
          <li v-for="comment in comments" :key="comment.id" class="flex flex-col gap-1">
            <p class="flex min-w-0 flex-wrap items-baseline gap-x-1 text-sm">
              <span class="font-medium text-highlighted">{{ comment.authorName }}</span>
              <span class="text-muted">· {{ timeText(comment.createdAt) }}</span>
              <button
                v-if="placeKind(comment) !== 'whole'"
                type="button"
                class="min-w-0 truncate rounded-sm px-1 font-mono text-toned transition-colors hover:bg-elevated active:bg-accented focus-inset"
                :title="$t('code.action.view_in_code', { place: placeOf(comment) })"
                @click="emit('go-comment', review, comment)"
              >
                {{ placeOf(comment) }}
              </button>
            </p>
            <pre
              v-if="comment.excerpt != null && review.revision !== props.revision"
              class="overflow-x-auto rounded-md bg-elevated p-2 font-mono text-sm whitespace-pre text-default"
            >{{ comment.excerpt }}</pre>
            <p class="text-sm break-words whitespace-pre-wrap text-highlighted">{{ comment.text }}</p>
          </li>
        </ol>

        <p v-if="sendFailure !== ''" class="px-3 text-sm text-error" role="alert">{{ sendFailure }}</p>

        <form class="flex flex-col gap-2 border-t border-default p-3" @submit.prevent="send">
          <UInput
            v-model="author"
            size="sm"
            icon="i-material-symbols-person-outline"
            :maxlength="AUTHOR_LIMIT"
            :placeholder="$t('code.placeholder.author')"
            :aria-label="$t('code.placeholder.author')"
          />
          <UTextarea
            v-model="text"
            :rows="2"
            size="sm"
            autoresize
            :maxrows="8"
            :maxlength="COMMENT_LIMIT"
            :placeholder="$t('code.placeholder.comment_whole')"
            :aria-label="$t('code.placeholder.comment_whole')"
            @keydown.meta.enter.prevent="send"
            @keydown.ctrl.enter.prevent="send"
          />
          <UButton
            type="submit"
            size="sm"
            class="self-end"
            icon="i-material-symbols-send-outline"
            :label="$t('code.shared.action.send')"
            :disabled="text.trim() === '' || author.trim() === ''"
            :loading="sending"
          />
        </form>

        <CodeReviewFiling v-model:open="filing" :review="review" :comments="comments" :folders="review.revision === props.revision ? props.folders : null" @filed="onFiled" />
      </template>
    </template>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { AUTHOR_LIMIT, COMMENT_LIMIT } from "../../shared/config";
import { placeKind as kindOf, placeText, reviewGroups, shortSha, STATE_COLORS, STATE_LABELS } from "../domain/code";
import { useAuthorName } from "../domain/kept";
import { refusalText } from "../domain/refusals";
import { changeReview, listComments, readReview, writeComment } from "../domain/review";
import { useReviewRefresh } from "../domain/events";
import { useNarrow } from "bmo-ui/app/domain/screen";

import type { ShownKind } from "../domain/code";
import type { Comment, Review } from "../../shared/types";

// The review part of the code review (docs/screen.md). The list is sorted out of the project's reviews the
// screen already holds; one review and its comments are read on their own and kept up by the service's events.

interface Props {
  reviews: Review[];
  // How the reading of those reviews stands: the display holds the place until the reading ends, and once it has
  // answered its place keeps what it answered, its nothing or its reason, while it is read again.
  loading: boolean;
  reviewsRead: boolean;
  failure: string;
  // The revision shown, and its folders, which tell a folder comment from a file comment.
  revision: string | null;
  folders: ReadonlySet<string> | null;
  openId: number | null;
  // Whether the part stands in a sheet whose own title already names the reviews.
  sheet?: boolean;
}

const { t, locale } = useI18n();
const toast = useToast();
const props = defineProps<Props>();
const emit = defineEmits<{
  open: [id: number];
  close: [];
  // A review moved to: the screen goes to its revision.
  go: [review: Review];
  // A comment's place gone to: the screen goes to its revision, file and line.
  "go-comment": [review: Review, comment: Comment];
  // A new review asked for, by a comment on the whole of it.
  new: [];
  // The open review changed here: a comment, a state, an item filed.
  changed: [];
  // The project's reviews read again, asked for under the reason its reading gave.
  retry: [];
}>();

const review = ref<Review | null>(null);
const comments = ref<Comment[]>([]);
const reviewLoading = ref(false);
const readFailure = ref("");
const text = ref("");
const author = useAuthorName();
const sending = ref(false);
const sendFailure = ref("");
const changing = ref(false);
const changeFailure = ref("");
const filing = ref(false);
const list = useTemplateRef<HTMLElement>("list");
// A phone by the layer's own measure, where the create button floats over the screen rather than standing here.
const phone = useNarrow();
let asked = 0;

const groups = computed(() => reviewGroups(props.reviews, props.revision));
const stateAction = computed(() => (review.value?.state === "closed" ? t("code.action.reopen_review") : t("code.action.close_review")));

// A time in the locale the console speaks.
const clock = computed(() => new Intl.DateTimeFormat(locale.value, { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" }));

function timeText(at: number): string {
  return clock.value.format(new Date(at));
}

// A comment's place, its folder told from a file by the revision's tree where the review is of the one shown.
function placeKind(comment: Comment): ShownKind {
  return kindOf(comment, review.value?.revision === props.revision ? props.folders : null);
}

function placeOf(comment: Comment): string {
  return placeText(t, comment, placeKind(comment));
}

// The open review and its comments. A reading that cannot be answered keeps its reason standing while it is read again, so only
// an answer takes it down.
async function load(): Promise<void> {
  const id = props.openId;

  if (id === null) {
    review.value = null;
    comments.value = [];
    readFailure.value = "";

    return;
  }

  const mine = ++asked;

  reviewLoading.value = true;

  try {
    const [read, written] = await Promise.all([readReview(id), listComments(id)]);

    if (mine === asked) {
      review.value = read;
      comments.value = written;
      readFailure.value = "";
    }
  } catch (error) {
    if (mine === asked) {
      readFailure.value = refusalText(t, error, t("code.error.review_read_failed"));
    }
  } finally {
    if (mine === asked) {
      reviewLoading.value = false;
    }
  }
}

async function send(): Promise<void> {
  const said = text.value.trim();
  const id = props.openId;

  if (said === "" || author.value.trim() === "" || id === null || sending.value) {
    return;
  }

  sending.value = true;
  sendFailure.value = "";

  try {
    await writeComment(id, { path: null, line: null, endLine: null, text: said, authorName: author.value.trim() });
    text.value = "";
    await load();
    emit("changed");
  } catch (error) {
    // Words the service refused were not written down, so they stay in the box to send again.
    sendFailure.value = refusalText(t, error, t("code.error.comment_post_failed"));
  } finally {
    sending.value = false;
  }
}

async function toggleState(): Promise<void> {
  const current = review.value;

  if (current === null || changing.value) {
    return;
  }

  changing.value = true;
  changeFailure.value = "";

  try {
    review.value = await changeReview(current.id, current.state === "closed" ? "open" : "closed");
    emit("changed");
  } catch (error) {
    changeFailure.value = refusalText(t, error, t("code.error.review_change_failed"));
  } finally {
    changing.value = false;
  }
}

async function onFiled(itemId: number): Promise<void> {
  toast.add({ title: t("code.message.item_filed", { id: itemId }), icon: "i-material-symbols-playlist-add-check" });
  await load();
  emit("changed");
}

watch(
  () => props.openId,
  () => {
    review.value = null;
    comments.value = [];
    text.value = "";
    sendFailure.value = "";
    changeFailure.value = "";
    filing.value = false;
    void load();
  },
  { immediate: true }
);

// The newest comment is the one last written, so the list goes to its foot when it grows.
watch(
  () => comments.value.length,
  async () => {
    await nextTick();

    if (list.value !== null) {
      list.value.scrollTop = list.value.scrollHeight;
    }
  }
);

useReviewRefresh(() => void load(), {
  on: ["review.changed", "comment.created", "filing.created"],
  where: (event) => event.reviewId === props.openId
});
</script>
