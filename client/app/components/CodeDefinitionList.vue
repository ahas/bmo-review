<template>
  <!-- Where a name is declared, when it stands in more than one place (docs/screen.md). One place
       goes straight there and none does nothing at all, so this list is only
       ever a choice to make, and it stands at the name itself at every width: a list to pick from
       is no part of the page lifted out. -->
  <UPopover v-bind="shell" :open="props.open" @update:open="emit('update:open', $event)">
    <template #content>
      <div class="flex min-h-0 flex-col">

        <div class="min-h-0 overflow-y-auto overscroll-contain">
          <button
            v-for="one in props.definitions"
            :key="`${one.path}:${one.line}`"
            type="button"
            class="flex w-full flex-col gap-1 border-b border-default px-2 py-2 text-start transition-colors last:border-b-0 hover:bg-elevated active:bg-accented focus-inset"
            @click="choose(one)"
          >
            <span class="flex w-full items-center gap-2 text-sm">
              <UBadge size="sm" color="neutral" variant="subtle" :label="$t(DEFINITION_LABELS[one.kind])" />
              <span class="min-w-0 flex-1 truncate font-mono text-toned">{{ one.path }}</span>
              <span class="shrink-0 font-mono tabular-nums text-muted">{{ one.line }}</span>
            </span>
            <span class="w-full truncate font-mono text-sm text-highlighted">{{ one.text.trim() }}</span>
          </button>
        </div>
      </div>
    </template>
  </UPopover>
</template>

<script setup lang="ts">
import { DEFINITION_LABELS } from "../domain/code";

import type { Definition } from "../../shared/types";

interface Props {
  open: boolean;
  definitions: readonly Definition[];
  // Where the name stands on the screen, which is what the list is put beside.
  at: { top: number; left: number; height: number } | null;
}

const props = defineProps<Props>();
const emit = defineEmits<{ "update:open": [open: boolean]; go: [definition: Definition] }>();

// The list stands at the name. What it is anchored to is the name's own place rather than an
// element, since the name is drawn by the editor and has no element of the console's to hang on.
const shell = computed(() => {
  const at = props.at;

  return {
    reference:
      at === null
        ? undefined
        : {
            getBoundingClientRect: () => ({
              x: at.left,
              y: at.top,
              top: at.top,
              left: at.left,
              right: at.left,
              bottom: at.top + at.height,
              width: 0,
              height: at.height,
              toJSON: () => ""
            })
          },
    content: { align: "start" as const, side: "bottom" as const },
    ui: { content: "w-[min(36rem,calc(100vw-2rem))] max-h-96" }
  };
});

function choose(one: Definition): void {
  emit("go", one);
}
</script>
