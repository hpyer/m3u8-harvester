<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';
import CommonIcon from '../ui/CommonIcon.vue';
import LocalFolderTree from './LocalFolderTree.vue';

const store = useAppStore();
</script>

<template>
  <div class="p-3 md:p-4">
    <div class="mb-3 flex flex-col items-start justify-between gap-2 md:mb-4 md:flex-row">
      <div class="flex flex-col gap-0.5 pt-0.5">
        <h2 class="flex items-center gap-2 text-base font-bold md:text-lg">
          本地文件
          <span class="badge badge-neutral badge-sm">{{ store.localFolders.length }}</span>
        </h2>
        <div
          v-if="store.downloadPath"
          class="max-w-[200px] truncate font-mono text-[10px] opacity-40 md:max-w-xl md:text-xs"
          :title="store.downloadPath"
        >
          {{ store.downloadPath.replace(/^.*\/storage\//, 'storage/') }}
        </div>
      </div>
      <button
        class="btn btn-xs btn-outline h-7 min-h-0 w-full gap-1 px-2.5 md:w-auto md:gap-1.5"
        @click="store.fetchLocalFiles()"
      >
        <CommonIcon name="refresh" class-name="h-3 w-3 md:h-3.5 md:w-3.5" />
        刷新目录
      </button>
    </div>

    <div v-if="store.localFolders.length === 0" class="py-12 text-center opacity-40 md:py-16">
      <div class="mb-3 text-4xl md:text-5xl">📂</div>
      <p class="text-sm">尚未发现已下载的视频文件</p>
    </div>

    <div v-else class="flex flex-col gap-2.5">
      <LocalFolderTree
        v-for="folder in store.localFolders"
        :key="folder.id"
        :folder="folder"
      />
    </div>
  </div>
</template>
