<template>
  <!-- One review in a list: where it is, how it opens, how long its thread is. -->
  <button
    type="button"
    class="flex min-h-11 w-full flex-col gap-1 rounded-md border border-default px-2 py-2 text-start transition-colors hover:bg-elevated active:bg-accented focus-inset"
    @click="emit('open')"
  >
    <span class="flex items-center gap-2 text-sm">
      <UBadge size="sm" color="neutral" variant="subtle" :label="$t(SCOPE_LABELS[props.review.scope])" />
      <span v-if="props.review.startLine !== null" class="font-mono text-toned">
        {{ props.review.startLine }}<template v-if="props.review.endLine !== props.review.startLine">–{{ props.review.endLine }}</template>
      </span>
      <span class="ms-auto flex items-center gap-1 text-muted">
        <UIcon name="i-material-symbols-chat-outline" class="size-4" />
        <span class="tabular-nums">{{ countText(props.review.turns) }}</span>
        <template v-if="props.review.todoIds.length > 0">
          · <UIcon name="i-material-symbols-checklist-rtl" class="size-4" /><span class="tabular-nums">{{ countText(props.review.todoIds.length) }}</span>
        </template>
      </span>
    </span>
    <span class="line-clamp-2 text-sm text-highlighted">{{ props.review.first }}</span>
    <span class="text-sm tabular-nums text-muted">{{ clock.format(new Date(props.review.lastAt * 1000)) }}</span>
  </button>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";
import { SCOPE_LABELS } from "../domain/code";

import type { CodeReviewSummary } from "../../shared/types";

const props = defineProps<{ review: CodeReviewSummary }>();
const emit = defineEmits<{ open: [] }>();

const clock = new Intl.DateTimeFormat("ko-KR", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
</script>
