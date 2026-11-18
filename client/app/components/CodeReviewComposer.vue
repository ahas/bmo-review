<template>
  <!-- The box a new comment is written in: where it goes, the review it goes into, the writer's name, the words,
       and the two ends of it. It stands between the code lines on a wide screen and inside a sheet everywhere
       else. -->
  <form class="flex flex-col gap-2 rounded-md border border-default bg-default p-2" @submit.prevent="submit" @keydown.esc.prevent="emit('cancel')">
    <p class="flex items-center gap-2 text-sm text-muted">
      <UIcon name="i-material-symbols-add-comment-outline" class="size-3.5 shrink-0" />
      <span class="min-w-0 truncate">
        <i18n-t :keypath="'code.message.pinned_to_revision'" tag="span" scope="global">
          <template #place><span class="font-mono text-highlighted">{{ props.place }}</span></template>
          <template #revision><span class="font-mono">{{ props.revision }}</span></template>
        </i18n-t>
      </span>
    </p>

    <div class="flex flex-wrap gap-2">
      <ChoiceSelect
        v-model="target"
        :items="targets"
        value-key="value"
        size="sm"
        :aria-label="$t('code.aria.choose_review')"
        class="min-w-40 flex-1"
      />
      <UInput
        v-model="author"
        size="sm"
        icon="i-material-symbols-person-outline"
        :maxlength="AUTHOR_LIMIT"
        :placeholder="$t('code.placeholder.author')"
        :aria-label="$t('code.placeholder.author')"
        class="min-w-32 flex-1"
      />
    </div>

    <UInput
      v-if="target === NEW"
      v-model="title"
      size="sm"
      :maxlength="TITLE_LIMIT"
      :placeholder="$t('code.placeholder.review_title')"
      :aria-label="$t('code.placeholder.review_title')"
    />

    <GrowingTextarea
      ref="box"
      v-model="text"
      :rows="3"
      size="sm"
      :maxlength="COMMENT_LIMIT"
      :placeholder="$t('code.placeholder.comment')"
      :aria-label="$t('code.placeholder.comment')"
      @keydown.meta.enter.prevent="submit"
      @keydown.ctrl.enter.prevent="submit"
    />

    <p v-if="props.failure !== ''" class="text-sm text-error" role="alert">{{ props.failure }}</p>

    <div class="flex items-center justify-end gap-2">
      <UButton size="sm" color="neutral" variant="ghost" :label="$t('code.shared.action.cancel')" @click="emit('cancel')" />
      <UButton type="submit" size="sm" icon="i-material-symbols-send-outline" :label="$t('code.shared.action.send')" :disabled="!ready" :loading="props.busy" />
    </div>
  </form>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { AUTHOR_LIMIT, COMMENT_LIMIT, TITLE_LIMIT } from "../../shared/config";
import { STATE_LABELS } from "../domain/code";
import { useAuthorName } from "../domain/kept";

import type { CommentDraft } from "../domain/review";
import type { Review } from "../../shared/types";

// Words the service refused stay in the box to send again; the screen empties it by taking it down.

interface Props {
  place: string;
  revision: string;
  // The reviews of the revision a comment may go into, and the one it goes into unless the reader picks another;
  // none is a new review opened by its title.
  reviews: Review[];
  target: number | null;
  busy: boolean;
  failure: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{ send: [draft: CommentDraft]; cancel: [] }>();
const { t } = useI18n();

// The choice that opens a new review.
const NEW = "new";

const text = ref("");
const title = ref("");
const target = ref<string>(props.target === null ? NEW : String(props.target));
const author = useAuthorName();
const box = useTemplateRef<{ focus: (options?: FocusOptions) => void }>("box");

const targets = computed(() => [
  ...props.reviews.map((review) => ({ value: String(review.id), label: `${review.title} · ${t(STATE_LABELS[review.state])}` })),
  { value: NEW, label: t("code.label.new_review") }
]);

const ready = computed(() => text.value.trim() !== "" && author.value.trim() !== "" && (target.value !== NEW || title.value.trim() !== ""));

function submit(): void {
  if (!ready.value || props.busy) {
    return;
  }

  emit("send", {
    reviewId: target.value === NEW ? null : Number(target.value),
    title: title.value.trim(),
    authorName: author.value.trim(),
    text: text.value.trim()
  });
}

onMounted(async () => {
  await nextTick();
  box.value?.focus({ preventScroll: true });
});
</script>
