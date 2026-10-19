<template>
  <!-- Filing a refactoring out of a review. What he writes is the title, what is to be done and,
       if he likes, what the talk came to; what the row will carry besides is stated as fact under
       it, and nothing about the row is approved here. A form of its own is no part of the page, so
       it stands in the middle of the screen at every width. -->
  <UModal :open="props.open" :ui="{ content: 'max-w-2xl' }" @update:open="emit('update:open', $event)">
    <template #content>
      <form class="flex max-h-full min-h-0 flex-col" @submit.prevent="submit">
        <div class="flex items-center gap-2 border-b border-default px-4 py-3">
          <UIcon name="i-material-symbols-playlist-add" class="size-4 shrink-0 text-muted" />
          <p class="min-w-0 flex-1 truncate text-sm font-medium text-highlighted">{{ $t("code.action.file_todo") }}</p>
          <UTooltip :text="$t('code.shared.action.close')">
            <UButton size="sm" color="neutral" variant="ghost" icon="i-material-symbols-close" :aria-label="$t('code.shared.action.close')" @click="emit('update:open', false)" />
          </UTooltip>
        </div>

        <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto overscroll-contain px-4 py-3">
          <UFormField :label="$t('code.label.todo_title')" required>
            <UInput v-model="title" size="sm" class="w-full" :maxlength="CODE_REVIEW_TITLE_LIMIT" />
          </UFormField>

          <UFormField :label="$t('code.label.todo_do')" required>
            <GrowingTextarea v-model="toDo" size="sm" :rows="6" :maxlength="CODE_REVIEW_TODO_LIMIT" />
          </UFormField>

          <UFormField :label="$t('code.label.todo_conclusion')">
            <GrowingTextarea v-model="conclusion" size="sm" :rows="2" :maxlength="CODE_REVIEW_CONCLUSION_LIMIT" />
          </UFormField>

          <dl class="pair-list gap-x-3 gap-y-1 rounded-md bg-elevated p-3 text-sm">
            <dt class="text-muted">{{ $t("code.label.todo_worker") }}</dt>
            <dd class="break-words text-highlighted">{{ props.workerLabel }}</dd>
            <dt class="text-muted">{{ $t("code.label.todo_place") }}</dt>
            <dd class="break-words font-mono text-highlighted">{{ reviewPlace($t, props.review) }}</dd>
            <dt class="text-muted">{{ $t("code.label.commit") }}</dt>
            <dd class="font-mono text-highlighted">{{ shortSha(props.review.commit) }}</dd>
            <dt class="text-muted">{{ $t("code.label.todo_conversation") }}</dt>
            <dd class="text-highlighted">{{ $t("code.label.turns", { n: countText(props.turns.length) }) }}</dd>
          </dl>

          <p v-if="failure !== ''" class="text-sm text-error" role="alert">{{ failure }}</p>
        </div>

        <div class="flex flex-wrap items-center justify-end gap-2 border-t border-default px-4 py-3">
          <UButton size="sm" color="neutral" variant="ghost" :label="$t('code.shared.action.cancel')" @click="emit('update:open', false)" />
          <UButton
            type="submit"
            size="sm"
            icon="i-material-symbols-playlist-add"
            :label="$t('code.action.file')"
            :loading="sending"
            :disabled="title.trim() === '' || toDo.trim() === '' || props.answering"
          />
        </div>
        <p v-if="props.answering" class="px-4 pb-3 text-end text-sm text-muted" role="status">
          {{ $t("code.status.file_after_answer") }}
        </p>
      </form>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";
import { CODE_REVIEW_CONCLUSION_LIMIT, CODE_REVIEW_TITLE_LIMIT, CODE_REVIEW_TODO_LIMIT } from "../../shared/config";
import { filingDraft, reviewPlace, shortSha } from "../domain/code";
import { refusalText } from "../domain/refusals";

import type { CodeReviewSummary, CodeReviewTurn } from "../../shared/types";

interface Props {
  open: boolean;
  review: CodeReviewSummary;
  turns: CodeReviewTurn[];
  workerLabel: string;
  answering: boolean;
}

const props = defineProps<Props>();
const { t } = useI18n();
const emit = defineEmits<{ "update:open": [open: boolean]; filed: [id: number] }>();


const title = ref("");
const toDo = ref("");
const conclusion = ref("");
const sending = ref(false);
const failure = ref("");

// Every opening starts from the thread as it stands now.
watch(
  () => props.open,
  (opened) => {
    if (!opened) {
      return;
    }

    const draft = filingDraft(t, props.review, props.turns);

    title.value = draft.title;
    toDo.value = draft.toDo;
    conclusion.value = "";
    failure.value = "";
  },
  { immediate: true }
);

async function submit(): Promise<void> {
  if (sending.value || props.answering || title.value.trim() === "" || toDo.value.trim() === "") {
    return;
  }

  sending.value = true;
  failure.value = "";

  try {
    const answer = await $fetch<{ ok: true; raised: number }>(`/api/code/reviews/${props.review.id}/todo`, {
      method: "POST",
      body: { title: title.value, toDo: toDo.value, conclusion: conclusion.value }
    });

    emit("filed", answer.raised);
    emit("update:open", false);
  } catch (error) {
    // What he wrote stays, so a refusal can be answered and sent again.
    failure.value = refusalText(t, error, t("code.error.file_failed"));
  } finally {
    sending.value = false;
  }
}
</script>
