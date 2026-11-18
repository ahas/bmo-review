<template>
  <!-- A new comment written away from the code lines: a file's, a folder's or the whole review's everywhere, and
       a line's on a narrow screen. A box to write in is no part of the page, so it stands in the layer's small
       dialog in the middle of the screen at every width: its head names the kind of comment, the writing box
       stands bare in its body, and cancel and send stand in its foot. Every way of closing it asks first while the
       box holds what it did not open with. -->
  <SettingsDialog :open="props.open" :title="$t(KIND_TITLES[props.kind])" :dirty="dirty" @update:open="emit('update:open', $event)">
    <!-- The box stands for as long as the dialog can be seen, a moment past the press that closed it, so what
         slides away is what was being written in and not an empty frame. A fresh box each time it opens: what
         was typed into the last one is not the next one's. -->
    <CodeReviewComposer
      v-if="standing"
      ref="composer"
      :key="openings"
      held
      :place="props.place"
      :revision="props.revision"
      :reviews="props.reviews"
      :target="props.target"
      :busy="props.busy"
      :failure="props.failure"
      @send="emit('send', $event)"
    />

    <template #footer="{ close }">
      <UButton color="neutral" variant="ghost" :label="$t('code.shared.action.cancel')" @click="close" />
      <UButton
        icon="i-material-symbols-send-outline"
        :label="$t('code.shared.action.send')"
        :disabled="composer?.ready !== true"
        :loading="props.busy"
        @click="composer?.submit()"
      />
    </template>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { useHeldWhileClosing } from "bmo-ui/app/domain/closing";

import type CodeReviewComposer from "./CodeReviewComposer.vue";
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
const composer = useTemplateRef<InstanceType<typeof CodeReviewComposer>>("composer");
// A box being sent, or one the dialog is closing on, holds nothing to ask about.
const dirty = computed(() => props.open && !props.busy && composer.value?.changes.dirty.value === true);

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
