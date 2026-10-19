<template>
  <!-- The box a new review is written in: where it goes, the words, and the two ends of it. It
       stands between the code lines on a wide screen and inside a sheet everywhere else. -->
  <form class="flex flex-col gap-2 rounded-md border border-default bg-default p-2" @submit.prevent="submit" @keydown.esc.prevent="emit('cancel')">
    <p class="flex items-center gap-2 text-sm text-muted">
      <UIcon name="i-material-symbols-add-comment-outline" class="size-3.5 shrink-0" />
      <span class="min-w-0 truncate">
        <i18n-t :keypath="'code.message.pinned_to_commit'" tag="span" scope="global">
          <template #place><span class="font-mono text-highlighted">{{ props.place }}</span></template>
          <template #commit><span class="font-mono">{{ props.commit }}</span></template>
        </i18n-t>
      </span>
    </p>

    <GrowingTextarea
      ref="box"
      v-model="text"
      :rows="3"
      size="sm"
      :maxlength="TALK_LIMIT"
      :placeholder="$t('code.title.review_compose')"
      :aria-label="$t('code.placeholder.review')"
      @keydown.meta.enter.prevent="submit"
      @keydown.ctrl.enter.prevent="submit"
    />

    <p v-if="props.failure !== ''" class="text-sm text-error" role="alert">{{ props.failure }}</p>

    <div class="flex items-center justify-end gap-2">
      <UButton size="sm" color="neutral" variant="ghost" :label="$t('code.shared.action.cancel')" @click="emit('cancel')" />
      <UButton type="submit" size="sm" icon="i-material-symbols-send-outline" :label="$t('code.action.add_review')" :disabled="text.trim() === ''" :loading="props.busy" />
    </div>
  </form>
</template>

<script setup lang="ts">
import { TALK_LIMIT } from "../../shared/config";

// Words the bot refused stay in the box to send again; the screen empties it by taking it down.

interface Props {
  place: string;
  commit: string;
  busy: boolean;
  failure: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{ send: [text: string]; cancel: [] }>();

const text = ref("");
const box = useTemplateRef<{ focus: (options?: FocusOptions) => void }>("box");

function submit(): void {
  if (text.value.trim() !== "" && !props.busy) {
    emit("send", text.value.trim());
  }
}

onMounted(async () => {
  await nextTick();
  box.value?.focus({ preventScroll: true });
});
</script>
