<template>
  <!-- One review in a list: its title, its state, who opened it, how many comments it holds and when it last
       moved. -->
  <button
    type="button"
    class="flex min-h-11 w-full flex-col gap-1 rounded-md border border-default px-2 py-2 text-start transition-colors hover:bg-elevated active:bg-accented focus-inset"
    @click="emit('open')"
  >
    <span class="flex items-center gap-2 text-sm">
      <UBadge size="sm" :color="STATE_COLORS[props.review.state]" variant="subtle" :label="$t(STATE_LABELS[props.review.state])" />
      <span class="min-w-0 truncate text-toned">{{ props.review.authorName }}</span>
      <span class="ms-auto flex items-center gap-1 text-muted">
        <UIcon name="i-material-symbols-chat-outline" class="size-4" />
        <span class="tabular-nums">{{ countText(props.review.commentCount) }}</span>
        <template v-if="props.review.itemIds.length > 0">
          · <UIcon name="i-material-symbols-checklist-rtl" class="size-4" /><span class="tabular-nums">{{ countText(props.review.itemIds.length) }}</span>
        </template>
      </span>
    </span>
    <span class="line-clamp-2 text-sm text-highlighted">{{ props.review.title }}</span>
    <span class="text-sm tabular-nums text-muted">{{ clock.format(new Date(props.review.changedAt)) }}</span>
  </button>
</template>

<script setup lang="ts">
import { countText } from "bmo-ui/app/domain/numbers";
import { useI18n } from "vue-i18n";
import { STATE_COLORS, STATE_LABELS } from "../domain/code";

import type { Review } from "../../shared/types";

const props = defineProps<{ review: Review }>();
const emit = defineEmits<{ open: [] }>();

const { locale } = useI18n();

// The change's time in the locale the console speaks.
const clock = computed(() => new Intl.DateTimeFormat(locale.value, { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" }));
</script>
