<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';
import type { FileInfo, FolderInfo } from '../../types/app';
import CommonIcon from '../ui/CommonIcon.vue';
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
  <div class="p-4 md:p-6">
    <div class="flex flex-col md:flex-row justify-between items-start gap-3 mb-4 md:mb-6">
      <div class="flex flex-col gap-0.5 pt-0.5">
        <h2 class="text-lg md:text-xl font-bold flex items-center gap-2">
          本地文件
          <span class="badge badge-neutral badge-sm">{{ store.localFolders.length }}</span>
        </h2>
        <div
          v-if="store.downloadPath"
          class="text-[10px] md:text-xs opacity-40 font-mono truncate max-w-[200px] md:max-w-xl"
          :title="store.downloadPath"
        >
          {{ store.downloadPath.replace(/^.*\/storage\//, 'storage/') }}
        </div>
      </div>
      <button
        class="btn btn-xs md:btn-sm btn-outline gap-1 md:gap-2 w-full md:w-auto h-8 min-h-0"
        @click="store.fetchLocalFiles()"
      >
        <CommonIcon name="refresh" class-name="h-3 w-3 md:h-3.5 md:w-3.5" />
        刷新目录
      </button>
    </div>

    <div v-if="store.localFolders.length === 0" class="text-center py-16 md:py-20 opacity-40">
      <div class="text-5xl md:text-6xl mb-4">📂</div>
      <p class="text-sm">尚未发现已下载的视频文件</p>
    </div>

    <div class="flex flex-col gap-3">
      <div
        v-for="folder in store.localFolders"
        :key="folder.id"
        class="collapse bg-base-200/40 border border-base-300 rounded-xl group overflow-visible"
      >
        <input type="checkbox" />

        <div class="collapse-title p-3 md:p-4 pr-24 md:pr-32">
          <div class="flex flex-col md:flex-row md:items-center gap-1 md:gap-4 mr-1">
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2 md:gap-3">
                <span class="text-lg md:text-xl grayscale opacity-70">📁</span>
                <div class="flex-1 min-w-0">
                  <div class="font-bold text-base md:text-lg truncate leading-tight">
                    {{ folder.name }}
                  </div>
                  <div
                    class="text-[10px] md:text-xs opacity-50 flex flex-wrap items-center gap-x-3 gap-y-0.5 mt-0.5"
                  >
                    <span class="font-medium">{{ folder.fileCount }} 个视频</span>
                    <span class="hidden md:inline opacity-30">|</span>
                    <span class="opacity-70">{{ folder.updatedAt }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div
          class="absolute right-2 top-3.5 md:top-5 z-20 flex items-center gap-0.5 md:gap-1"
          @click.stop
        >
          <button
            class="btn btn-ghost btn-circle btn-xs md:btn-sm hover:bg-base-300/50"
            @click="renameFolder(folder)"
          >
            <CommonIcon name="edit" class-name="h-3.5 w-3.5 md:h-4 md:w-4" />
          </button>
          <button
            class="btn btn-ghost btn-circle btn-xs md:btn-sm text-error hover:bg-error/10"
            @click="store.deleteFolder(folder.id)"
          >
            <CommonIcon name="trash" class-name="h-3.5 w-3.5 md:h-4 md:w-4" />
          </button>
          <div class="btn btn-ghost btn-circle btn-xs md:btn-sm pointer-events-none opacity-40">
            <CommonIcon
              name="chevron-down"
              class-name="h-3.5 w-3.5 md:h-5 md:w-5 transition-transform duration-300 peer-checked:rotate-180"
            />
          </div>
        </div>

        <div
          class="collapse-content px-0 pt-0 bg-base-100/30 rounded-b-xl border-t border-base-300/50"
        >
          <!-- Desktop View -->
          <div class="hidden md:block overflow-x-auto">
            <table class="table table-sm w-full">
              <thead>
                <tr class="bg-base-200/50">
                  <th class="pl-6 py-3 text-[10px] uppercase tracking-wider opacity-60">文件名</th>
                  <th class="w-32 py-3 text-[10px] uppercase tracking-wider opacity-60">大小</th>
                  <th class="w-48 py-3 text-[10px] uppercase tracking-wider opacity-60">
                    修改时间
                  </th>
                  <th class="w-32 py-3 text-center text-[10px] uppercase tracking-wider opacity-60">
                    操作
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="file in folder.files"
                  :key="file.id"
                  class="hover:bg-base-200/30 border-b border-base-200 last:border-0"
                >
                  <td class="pl-6">
                    <div class="flex items-center gap-2 py-2.5">
                      <span class="text-base grayscale opacity-70">🎬</span>
                      <span class="truncate max-w-xs font-medium" :title="file.name">{{
                        file.name
                      }}</span>
                    </div>
                  </td>
                  <td class="text-xs opacity-60 font-mono">{{ file.size }}</td>
                  <td class="text-xs opacity-60 font-mono">{{ file.updatedAt }}</td>
                  <td class="text-center">
                    <div class="flex items-center justify-center gap-1">
                      <button
                        class="btn btn-ghost btn-xs btn-circle hover:bg-base-300/50"
                        @click="renameFile(file)"
                      >
                        <CommonIcon name="edit" class-name="h-3 w-3" />
                      </button>
                      <button
                        class="btn btn-ghost btn-xs btn-circle text-error hover:bg-error/10"
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

          <!-- Mobile View -->
          <div class="md:hidden divide-y divide-base-300/30">
            <div
              v-for="file in folder.files"
              :key="file.id"
              class="p-4 flex flex-col gap-2 transition-colors duration-200"
            >
              <div class="flex items-start gap-2.5">
                <span class="text-base grayscale opacity-70 shrink-0 mt-0.5">🎬</span>
                <span class="text-[13px] font-semibold break-all flex-1 leading-snug">{{
                  file.name
                }}</span>
              </div>
              <div class="flex justify-between items-center pl-8 mt-1">
                <div class="flex flex-col gap-0.5">
                  <span class="text-[9px] opacity-40 font-mono uppercase tracking-tighter"
                    >Size: {{ file.size }}</span
                  >
                  <span class="text-[9px] opacity-40 font-mono uppercase tracking-tighter"
                    >Date: {{ file.updatedAt }}</span
                  >
                </div>
                <div class="flex gap-0.5">
                  <button
                    class="btn btn-ghost btn-circle btn-sm h-8 w-8 hover:bg-base-300/50"
                    @click="renameFile(file)"
                  >
                    <CommonIcon name="edit" class-name="h-3.5 w-3.5" />
                  </button>
                  <button
                    class="btn btn-ghost btn-circle btn-sm text-error h-8 w-8 hover:bg-error/10"
                    @click="store.deleteFile(file.id)"
                  >
                    <CommonIcon name="trash" class-name="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
