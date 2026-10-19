<template>
  <!-- A new review written away from the code lines: a file's or a folder's everywhere, and a
       line's on a narrow screen. A box to write in is no part of the page, so it stands in the
       middle of the screen at every width. -->
  <UModal :open="props.open" :ui="{ content: 'max-w-xl' }" @update:open="emit('update:open', $event)">
    <template #content>
      <div class="flex flex-col gap-3 p-4">
        <p class="text-sm font-medium text-highlighted">{{ SCOPE_TITLES[props.scope] }}</p>
        <!-- The box stands for as long as the dialog can be seen, a moment past the press that
             closed it, so what slides away is what was being written in and not an empty frame. A
             fresh box each time it opens: what was typed into the last one is not the next one's. -->
        <CodeReviewComposer
          v-if="standing"
          :key="openings"
          :place="props.place"
          :commit="props.commit"
          :busy="props.busy"
          :failure="props.failure"
          @send="emit('send', $event)"
          @cancel="emit('update:open', false)"
        />
      </div>
    </template>
  </UModal>
</template>

<script setup lang="ts">
import { useHeldWhileClosing } from "bmo-ui/app/domain/closing";

import type { CodeReviewScope } from "../../shared/types";

interface Props {
  open: boolean;
  scope: CodeReviewScope;
  place: string;
  commit: string;
  busy: boolean;
  failure: string;
}

const { t } = useI18n();
const props = defineProps<Props>();
const emit = defineEmits<{ "update:open": [open: boolean]; send: [text: string] }>();

const standing = useHeldWhileClosing(() => (props.open ? true : null));
const openings = ref(0);

watch(
  () => props.open,
  (open) => {
    if (open) openings.value += 1;
  },
  { immediate: true }
);

const SCOPE_TITLES: Record<CodeReviewScope, string> = {
  line: t("code.label.review_line"),
  range: t("code.label.review_range"),
  file: t("code.label.review_file"),
  folder: t("code.label.review_folder")
};
</script>
