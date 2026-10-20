<template>
  <!-- Filing an item from a review into the job service. What the reader writes is the title, the brief and the
       queue it goes into; the review it comes from is stated as fact under it, and everything else about the
       item is the job service's. A form of its own is no part of the page, so it stands in the middle of the
       screen at every width. -->
  <UModal :open="props.open" :ui="{ content: 'max-w-2xl' }" @update:open="emit('update:open', $event)">
    <template #content>
      <form class="flex max-h-full min-h-0 flex-col" @submit.prevent="submit">
        <div class="flex items-center gap-2 border-b border-default px-4 py-3">
          <UIcon name="i-material-symbols-playlist-add" class="size-4 shrink-0 text-muted" />
          <p class="min-w-0 flex-1 truncate text-sm font-medium text-highlighted">{{ $t("code.action.file_item") }}</p>
          <UTooltip :text="$t('code.shared.action.close')">
            <UButton size="sm" color="neutral" variant="ghost" icon="i-material-symbols-close" :aria-label="$t('code.shared.action.close')" @click="emit('update:open', false)" />
          </UTooltip>
        </div>

        <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-4 py-3">
          <UFormField :label="$t('code.label.item_title')" required>
            <UInput v-model="title" size="sm" class="w-full" :maxlength="TITLE_LIMIT" />
          </UFormField>

          <UFormField :label="$t('code.label.item_brief')" required>
            <GrowingTextarea v-model="brief" size="sm" :rows="8" :maxlength="BRIEF_LIMIT" />
          </UFormField>

          <UFormField :label="$t('code.label.item_queue')" required>
            <UInput v-model="queue" size="sm" class="w-full" :placeholder="$t('code.placeholder.queue')" />
          </UFormField>

          <dl class="pair-list gap-x-3 gap-y-1 rounded-md bg-elevated p-3 text-sm">
            <dt class="text-muted">{{ $t("code.label.review") }}</dt>
            <dd class="break-words text-highlighted">{{ props.review.title }}</dd>
            <dt class="text-muted">{{ $t("code.label.project") }}</dt>
            <dd class="break-words text-highlighted">{{ props.review.projectName }}</dd>
            <dt class="text-muted">{{ $t("code.label.revision") }}</dt>
            <dd class="font-mono text-highlighted">{{ shortSha(props.review.revision) }}</dd>
            <dt class="text-muted">{{ $t("code.label.comments") }}</dt>
            <dd class="text-highlighted tabular-nums">{{ countText(props.comments.length) }}</dd>
          </dl>

          <p v-if="failure !== ''" class="text-sm text-error" role="alert">{{ failure }}</p>
        </div>

        <div class="flex flex-wrap items-center justify-end gap-2 border-t border-default px-4 py-3">
          <UButton size="sm" color="neutral" variant="ghost" :label="$t('code.shared.action.cancel')" @click="emit('update:open', false)" />
          <UButton type="submit" size="sm" icon="i-material-symbols-playlist-add" :label="$t('code.action.file')" :loading="sending" :disabled="!ready" />
        </div>
      </form>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { countText } from "bmo-ui/app/domain/numbers";
import { BRIEF_LIMIT, TITLE_LIMIT } from "../../shared/config";
import { filingDraft, shortSha } from "../domain/code";
import { useQueueName } from "../domain/kept";
import { refusalText } from "../domain/refusals";
import { fileItem } from "../domain/review";

import type { Comment, Review } from "../../shared/types";

interface Props {
  open: boolean;
  review: Review;
  comments: Comment[];
  // The folders of the revision being read, where it is the review's, so a comment's place is called a folder
  // or a file rightly.
  folders: ReadonlySet<string> | null;
}

const props = defineProps<Props>();
const { t } = useI18n();
const emit = defineEmits<{ "update:open": [open: boolean]; filed: [itemId: number] }>();

const title = ref("");
const brief = ref("");
const queue = useQueueName();
const sending = ref(false);
const failure = ref("");

const ready = computed(() => title.value.trim() !== "" && brief.value.trim() !== "" && queue.value.trim() !== "");

// Every opening starts from the review as it stands now.
watch(
  () => props.open,
  (opened) => {
    if (!opened) {
      return;
    }

    const draft = filingDraft(t, props.review, props.comments, props.folders, BRIEF_LIMIT);

    title.value = draft.title.slice(0, TITLE_LIMIT);
    brief.value = draft.brief;
    failure.value = "";
  },
  { immediate: true }
);

async function submit(): Promise<void> {
  if (sending.value || !ready.value) {
    return;
  }

  sending.value = true;
  failure.value = "";

  try {
    const filing = await fileItem(props.review.id, { title: title.value.trim(), brief: brief.value.trim(), queueName: queue.value.trim() });

    emit("filed", filing.itemId);
    emit("update:open", false);
  } catch (error) {
    // What was written stays, so a refusal can be answered and sent again.
    failure.value = refusalText(t, error, t("code.error.file_failed"));
  } finally {
    sending.value = false;
  }
}
</script>
