<script setup lang="ts">
// The dev host's own frame: the app root the layer asks its host for, the overlay provider the screen's dialogs
// open through, the one place news is said, and the screen's page rendering inside it.
import { ko } from "@nuxt/ui/locale";
import MessageIsland from "bmo-ui/app/components/MessageIsland.vue";
import { TOOLTIP_PROVIDER } from "bmo-ui/app/domain/tooltip";
</script>

<template>
  <UApp :locale="ko" :tooltip="TOOLTIP_PROVIDER" :toaster="null">
    <div class="flex h-dvh min-h-0 flex-col">
      <NuxtPage />
    </div>

    <MessageIsland />
  </UApp>
</template>
