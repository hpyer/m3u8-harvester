<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { useAppStore } from '../../stores/appStore';
import CommonIcon from '../ui/CommonIcon.vue';

const store = useAppStore();
const { versionInfo } = storeToRefs(store);
</script>

<template>
  <footer
    class="footer items-center p-2 md:p-4 bg-base-100 text-base-content border-t border-base-200 flex flex-col md:flex-row justify-between gap-2 md:gap-4"
  >
    <div class="flex items-center gap-2 md:gap-3">
      <div class="flex items-center gap-1.5">
        <span class="text-base md:text-lg">📦</span>
        <span class="font-bold text-xs md:text-sm">M3U8 Harvester</span>
      </div>
      <span class="badge badge-xs badge-ghost opacity-50 scale-90"
        >Server v{{ versionInfo.serverVersion }}</span
      >
      <span class="badge badge-xs badge-ghost opacity-50 scale-90"
        >Web v{{ versionInfo.webVersion }}</span
      >
      <span class="text-[9px] md:text-[10px] opacity-30">© 2026 All rights reserved.</span>
    </div>

    <div
      class="flex items-center gap-2 text-[9px] md:text-[10px] opacity-60 bg-base-200/50 px-2 md:px-3 py-1 rounded-lg border border-base-300 max-w-xs md:max-w-sm"
    >
      <CommonIcon name="warning" class-name="h-3 w-3 md:h-3.5 md:w-3.5 text-warning shrink-0" />
      <p class="leading-tight">本项目仅供学习，下载受版权保护的内容前请确保已获得授权。</p>
    </div>
  </footer>
</template>
