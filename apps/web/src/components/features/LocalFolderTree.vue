<script setup lang="ts">
defineOptions({ name: 'LocalFolderTree' });

import type { FileInfo, FolderInfo } from '../../types/app';
import { useAppStore } from '../../stores/appStore';
import CommonIcon from '../ui/CommonIcon.vue';

const props = withDefaults(
  defineProps<{
    folder: FolderInfo;
    nested?: boolean;
  }>(),
  {
    nested: false,
  },
);

const store = useAppStore();

const renameFolder = (folder: FolderInfo) => {
  const newName = prompt('重命名文件夹', folder.name);
  if (newName && newName !== folder.name) {
    store.renameFileOrFolder(folder.id, newName);
  }
};

const renameFile = (file: FileInfo) => {
  const newName = prompt('重命名文件', file.name);
  if (newName && newName !== file.name) {
    store.renameFileOrFolder(file.id, newName);
  }
};
</script>

<template>
  <details
    class="group rounded-lg border border-base-300 bg-base-200/35 open:bg-base-200/55"
    :open="!nested"
  >
    <summary
      class="flex cursor-pointer list-none items-center justify-between gap-2.5 px-3 py-2.5 md:px-3.5 md:py-3"
      :class="nested ? 'pl-3.5 md:pl-4.5' : ''"
    >
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2">
          <span class="text-base md:text-lg grayscale opacity-70">📁</span>
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-bold leading-tight md:text-base">
              {{ props.folder.name }}
            </div>
            <div
              class="mt-0.5 flex flex-wrap items-center gap-x-2.5 gap-y-0.5 text-[10px] opacity-50"
            >
              <span class="font-medium">{{ props.folder.fileCount }} 个视频</span>
              <span class="hidden opacity-30 md:inline">|</span>
              <span class="opacity-70">{{ props.folder.updatedAt }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="flex items-center gap-0.5" @click.stop>
        <button
          class="btn btn-ghost btn-circle btn-xs h-7 min-h-0 w-7 hover:bg-base-300/50"
          @click="renameFolder(props.folder)"
        >
          <CommonIcon name="edit" class-name="h-3.5 w-3.5" />
        </button>
        <button
          class="btn btn-ghost btn-circle btn-xs h-7 min-h-0 w-7 text-error hover:bg-error/10"
          @click="store.deleteFolder(props.folder.id)"
        >
          <CommonIcon name="trash" class-name="h-3.5 w-3.5" />
        </button>
        <CommonIcon
          name="chevron-down"
          class-name="h-4 w-4 opacity-40 transition-transform duration-300 group-open:rotate-180"
        />
      </div>
    </summary>

    <div class="border-t border-base-300/50 bg-base-100/20 px-2.5 py-2 md:px-3 md:py-2.5">
      <div v-if="props.folder.folders.length" class="flex flex-col gap-1.5">
        <LocalFolderTree
          v-for="child in props.folder.folders"
          :key="child.id"
          :folder="child"
          nested
        />
      </div>

      <div
        v-if="props.folder.files.length"
        class="mt-1.5 overflow-hidden rounded-md border border-base-300/40"
      >
        <div class="hidden overflow-x-auto md:block">
          <table class="table table-sm w-full">
            <thead>
              <tr class="bg-base-200/50">
                <th class="pl-4 py-2 text-[10px] uppercase tracking-wider opacity-60">文件名</th>
                <th class="w-28 py-2 text-[10px] uppercase tracking-wider opacity-60">大小</th>
                <th class="w-40 py-2 text-[10px] uppercase tracking-wider opacity-60">修改时间</th>
                <th class="w-24 py-2 text-center text-[10px] uppercase tracking-wider opacity-60">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="file in props.folder.files"
                :key="file.id"
                class="border-b border-base-200 hover:bg-base-200/30 last:border-0"
              >
                <td class="pl-4">
                  <div class="flex items-center gap-2 py-1.5">
                    <span class="text-sm grayscale opacity-70">🎬</span>
                    <span class="max-w-xs truncate font-medium" :title="file.name">{{ file.name }}</span>
                  </div>
                </td>
                <td class="text-xs opacity-60 font-mono">{{ file.size }}</td>
                <td class="text-xs opacity-60 font-mono">{{ file.updatedAt }}</td>
                <td class="text-center">
                  <div class="flex items-center justify-center gap-1">
                    <button
                      class="btn btn-ghost btn-xs btn-circle h-6 min-h-0 w-6 hover:bg-base-300/50"
                      @click="renameFile(file)"
                    >
                      <CommonIcon name="edit" class-name="h-3 w-3" />
                    </button>
                    <button
                      class="btn btn-ghost btn-xs btn-circle h-6 min-h-0 w-6 text-error hover:bg-error/10"
                      @click="store.deleteFile(file.id)"
                    >
                      <CommonIcon name="trash" class-name="h-3 w-3" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="divide-y divide-base-300/30 md:hidden">
          <div
            v-for="file in props.folder.files"
            :key="file.id"
            class="flex flex-col gap-1.5 px-3 py-2.5 transition-colors duration-200"
          >
            <div class="flex items-start gap-2">
              <span class="mt-0.5 shrink-0 text-sm grayscale opacity-70">🎬</span>
              <span class="flex-1 break-all text-[12px] font-semibold leading-snug">{{ file.name }}</span>
            </div>
            <div class="mt-0.5 flex items-center justify-between pl-6">
              <div class="flex flex-col gap-0.5">
                <span class="text-[9px] uppercase tracking-tighter opacity-40 font-mono">Size: {{ file.size }}</span>
                <span class="text-[9px] uppercase tracking-tighter opacity-40 font-mono">Date: {{ file.updatedAt }}</span>
              </div>
              <div class="flex gap-0.5">
                <button
                  class="btn btn-ghost btn-circle btn-xs h-7 min-h-0 w-7 hover:bg-base-300/50"
                  @click="renameFile(file)"
                >
                  <CommonIcon name="edit" class-name="h-3 w-3" />
                </button>
                <button
                  class="btn btn-ghost btn-circle btn-xs h-7 min-h-0 w-7 text-error hover:bg-error/10"
                  @click="store.deleteFile(file.id)"
                >
                  <CommonIcon name="trash" class-name="h-3 w-3" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </details>
</template>
