<template>
  <!-- A new comment written away from the code lines: a file's, a folder's or the whole review's everywhere, and
       a line's on a narrow screen. A box to write in is no part of the page, so it stands in the middle of the
       screen at every width. -->
  <UModal :open="props.open" :ui="{ content: 'max-w-xl' }" @update:open="emit('update:open', $event)">
    <template #content>
      <div class="flex flex-col gap-3 p-4">
        <p class="text-sm font-medium text-highlighted">{{ $t(KIND_TITLES[props.kind]) }}</p>
        <!-- The box stands for as long as the dialog can be seen, a moment past the press that closed it, so
             what slides away is what was being written in and not an empty frame. A fresh box each time it
             opens: what was typed into the last one is not the next one's. -->
        <CodeReviewComposer
          v-if="standing"
          :key="openings"
          :place="props.place"
          :revision="props.revision"
          :reviews="props.reviews"
          :target="props.target"
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

import type { PlaceKind } from "../domain/code";
import type { CommentDraft } from "../domain/review";
import type { Review } from "../../shared/types";

interface Props {
  open: boolean;
  kind: PlaceKind;
  place: string;
  revision: string;
  reviews: Review[];
  target: number | null;
  busy: boolean;
  failure: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{ "update:open": [open: boolean]; send: [draft: CommentDraft] }>();

const standing = useHeldWhileClosing(() => (props.open ? true : null));
const openings = ref(0);

watch(
  () => props.open,
  (open) => {
    if (open) openings.value += 1;
  },
  { immediate: true }
);

const KIND_TITLES: Record<PlaceKind, string> = {
  whole: "code.label.comment_whole",
  line: "code.label.comment_line",
  range: "code.label.comment_range",
  file: "code.label.comment_file",
  folder: "code.label.comment_folder"
};
</script>
