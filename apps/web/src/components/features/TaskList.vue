<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';
import type { TaskCategory, TaskGroup, TaskStatus } from '../../types/app';
import {
  TASK_CATEGORY_LABELS,
  TASK_STATUS_BADGE_CLASS,
  TASK_STATUS_LABELS,
  isPausableParentStatus,
  isPausableSubtaskStatus,
  isResumableStatus,
} from '../../constants/ui';
import CommonIcon from '../ui/CommonIcon.vue';
const store = useAppStore();

const getStatusBadgeClass = (status: TaskStatus) => TASK_STATUS_BADGE_CLASS[status];
const getStatusLabel = (status: TaskStatus) => TASK_STATUS_LABELS[status];
const getCategoryLabel = (cat: TaskCategory) => TASK_CATEGORY_LABELS[cat];

const formatPercentage = (p: number | null | undefined) => {
  if (p === undefined || p === null) return '0.0';
  return Number(p).toFixed(1);
};

const formatSize = (bytes?: number | null) => {
  if (!bytes) return '-';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let size = bytes;
  let unitIndex = 0;
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }
  return `${size.toFixed(1)} ${units[unitIndex]}`;
};

const getCompletedSubtasks = (task: TaskGroup) => {
  return task.subtasks.filter((t) => ['completed', 'skipped'].includes(t.status)).length;
};

const copyToClipboard = (text: string | null | undefined) => {
  if (!text) return;
  navigator.clipboard
    .writeText(text)
    .then(() => {
      // 简单的成功提示
      console.log('Copied to clipboard');
    })
    .catch((err) => {
      console.error('Failed to copy: ', err);
      alert('复制失败');
    });
};
</script>

<template>
  <div class="p-4 md:p-6">
    <div class="flex flex-col md:flex-row justify-between items-start gap-3 mb-4 md:mb-6">
      <h2 class="text-lg md:text-xl font-bold flex items-center gap-2 pt-0.5">
        任务列表
        <span class="badge badge-neutral badge-sm">{{ store.tasks.length }}</span>
      </h2>
      <div class="flex items-center gap-2 w-full md:w-auto">
        <div
          v-if="store.pollTimer"
          class="hidden md:flex items-center gap-2 text-xs text-success animate-pulse mr-2"
        >
          <span class="relative flex h-2 w-2">
            <span
              class="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-75"
            ></span>
            <span class="relative inline-flex rounded-full h-2 w-2 bg-success"></span>
          </span>
          正在自动刷新
        </div>
        <button
          class="btn btn-xs md:btn-sm btn-primary gap-1 md:gap-2 flex-1 md:flex-none h-8 min-h-0"
          @click="store.openAddTaskModal()"
        >
          <CommonIcon name="plus" class-name="h-3 w-3 md:h-3.5 md:w-3.5" />
          添加任务
        </button>
        <button
          class="btn btn-xs md:btn-sm btn-outline gap-1 md:gap-2 flex-1 md:flex-none h-8 min-h-0"
          @click="store.fetchTasks()"
        >
          <CommonIcon name="refresh" class-name="h-3 w-3 md:h-3.5 md:w-3.5" />
          刷新列表
        </button>
      </div>
    </div>

    <div v-if="store.tasks.length === 0" class="text-center py-16 md:py-20 opacity-40">
      <div class="text-5xl md:text-6xl mb-4">📥</div>
      <p class="text-sm">暂无活跃任务，点击上方“添加任务”开始下载</p>
    </div>

    <div v-else class="flex flex-col gap-3">
      <div
        v-for="task in store.tasks"
        :key="task.id"
        class="collapse bg-base-200/40 border border-base-300 rounded-xl group relative overflow-visible"
      >
        <!-- 将 checkbox 放在最前面，但按钮需要绝对定位或在 title 外部以防被拦截 -->
        <input type="checkbox" checked class="peer" />

        <div class="collapse-title p-2 md:p-4 pr-32 md:pr-44">
          <div class="flex flex-col md:flex-row md:items-center gap-1 md:gap-4 mr-1">
            <div class="flex-1 min-w-0">
              <div class="flex flex-wrap items-center gap-1.5 mb-0.5 md:mb-1">
                <span class="font-bold text-base md:text-lg truncate max-w-[140px] md:max-w-none">{{
                  task.groupTitle || task.title
                }}</span>
                <span class="badge badge-sm badge-outline opacity-70">{{
                  getCategoryLabel(task.type)
                }}</span>
                <span :class="['badge badge-sm badge-outline', getStatusBadgeClass(task.status)]">{{
                  getStatusLabel(task.status)
                }}</span>
              </div>
              <div class="flex items-center gap-2">
                <span
                  class="text-[10px] md:text-xs opacity-50 uppercase tracking-wider font-semibold"
                  >任务：</span
                >
                <span class="text-[10px] md:text-xs font-mono font-bold text-primary"
                  >{{ getCompletedSubtasks(task) }} / {{ task.subtasks?.length || 0 }}</span
                >
              </div>
            </div>
          </div>
        </div>

        <!-- 绝对定位的操作按钮 -->
        <div class="absolute right-2 top-2.5 md:top-5 z-20 flex items-center gap-0.5 md:gap-1">
          <button
            class="btn btn-ghost btn-circle btn-xs md:btn-sm text-primary hover:bg-primary/10"
            @click.stop="
              store.openAddTaskModal({
                title: task.groupTitle || task.title,
                category: task.type,
                year: task.year ?? '',
                season: task.season ?? '',
              })
            "
          >
            <CommonIcon name="plus" class-name="h-3.5 w-3.5 md:h-5 md:w-5" />
          </button>
          <button
            v-if="isResumableStatus(task.status)"
            class="btn btn-ghost btn-circle btn-xs md:btn-sm text-success hover:bg-success/10"
            @click.stop="store.resumeTask(task.id)"
          >
            <CommonIcon name="play" class-name="h-3.5 w-3.5 md:h-5 md:w-5" />
          </button>
          <button
            v-else-if="isPausableParentStatus(task.status)"
            class="btn btn-ghost btn-circle btn-xs md:btn-sm text-warning hover:bg-warning/10"
            @click.stop="store.pauseTask(task.id)"
          >
            <CommonIcon name="pause" class-name="h-3.5 w-3.5 md:h-5 md:w-5" />
          </button>
          <button
            class="btn btn-ghost btn-circle btn-xs md:btn-sm text-error hover:bg-error/10"
            @click.stop="store.deleteTask(task.id)"
          >
            <CommonIcon name="trash" class-name="h-3.5 w-3.5 md:h-5 md:w-5" />
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
                  <th class="pl-6 py-3 text-[10px] uppercase tracking-wider opacity-60">
                    文件名 & 状态
                  </th>
                  <th class="w-64 py-3 text-[10px] uppercase tracking-wider opacity-60">进度</th>
                  <th class="w-24 py-3 text-[10px] uppercase tracking-wider opacity-60">大小</th>
                  <th class="w-32 py-3 text-center text-[10px] uppercase tracking-wider opacity-60">
                    操作
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="sub in task.subtasks"
                  :key="sub.id"
                  class="hover:bg-base-200/30 border-b border-base-200 last:border-0"
                >
                  <td class="pl-6">
                    <div class="flex items-center gap-2 py-2">
                      <span class="text-xs">📄</span>
                      <span class="truncate max-w-xs font-medium" :title="sub.title">{{
                        sub.title
                      }}</span>
                      <span
                        :class="[
                          'badge badge-sm badge-outline font-semibold py-1.5 px-2 scale-90',
                          getStatusBadgeClass(sub.status),
                        ]"
                      >
                        {{ getStatusLabel(sub.status) }}
                      </span>
                    </div>
                  </td>
                  <td>
                    <div class="flex flex-col py-2">
                      <div class="flex justify-between items-end mb-0.5 px-0.5">
                        <span class="text-[9px] opacity-40 font-mono uppercase tracking-tighter"
                          >{{ sub.completedSegments }} / {{ sub.totalSegments }}</span
                        >
                        <span class="text-[10px] font-mono font-bold text-primary"
                          >{{ formatPercentage(sub.percentage) }}%</span
                        >
                      </div>
                      <progress
                        class="progress progress-primary w-full h-1 shadow-inner bg-base-300"
                        :value="sub.percentage"
                        max="100"
                      ></progress>
                    </div>
                  </td>
                  <td>
                    <div class="text-[11px] font-mono opacity-60">
                      {{ formatSize(sub.estimatedSize) }}
                    </div>
                  </td>
                  <td class="text-center">
                    <div class="flex justify-center gap-1">
                      <button
                        class="btn btn-ghost btn-xs text-info"
                        title="复制下载链接"
                        @click="copyToClipboard(sub.m3u8Url)"
                      >
                        <CommonIcon name="copy" class-name="h-4 w-4" />
                      </button>
                      <button
                        v-if="isResumableStatus(sub.status)"
                        class="btn btn-ghost btn-xs text-success"
                        @click="store.resumeTask(sub.id)"
                      >
                        <CommonIcon name="play" class-name="h-4 w-4" />
                      </button>
                      <button
                        v-else-if="isPausableSubtaskStatus(sub.status)"
                        class="btn btn-ghost btn-xs text-warning"
                        @click="store.pauseTask(sub.id)"
                      >
                        <CommonIcon name="pause" class-name="h-4 w-4" />
                      </button>
                      <button
                        class="btn btn-ghost btn-xs text-error"
                        @click="store.deleteTask(sub.id)"
                      >
                        <CommonIcon name="trash" class-name="h-4 w-4" />
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
              v-for="sub in task.subtasks"
              :key="sub.id"
              class="p-2 px-4 flex flex-col gap-1 transition-colors duration-200"
            >
              <div class="flex justify-between items-start gap-2">
                <div class="flex flex-wrap items-center gap-2 min-w-0">
                  <span class="text-xs grayscale opacity-70 shrink-0">📄</span>
                  <span
                    class="text-[13px] font-semibold truncate leading-tight"
                    :title="sub.title"
                    >{{ sub.title }}</span
                  >
                  <span
                    :class="[
                      'badge badge-sm badge-outline font-bold py-1 px-1.5 scale-90 origin-left',
                      getStatusBadgeClass(sub.status),
                    ]"
                  >
                    {{ getStatusLabel(sub.status) }}
                  </span>
                </div>
              </div>

              <div class="flex items-center gap-3">
                <div class="flex-1">
                  <div class="flex justify-between items-end mb-0.5 px-0.5">
                    <span class="text-[9px] opacity-50 font-mono uppercase tracking-tighter"
                      >{{ sub.completedSegments }} / {{ sub.totalSegments }}</span
                    >
                    <span class="text-[10px] font-mono font-bold text-primary"
                      >{{ formatPercentage(sub.percentage) }}%</span
                    >
                  </div>
                  <progress
                    class="progress progress-primary w-full h-1 shadow-inner bg-base-300"
                    :value="sub.percentage"
                    max="100"
                  ></progress>
                </div>

                <div class="flex gap-0.5 shrink-0">
                  <button
                    class="btn btn-ghost btn-circle btn-sm text-info h-7 w-7"
                    title="复制下载链接"
                    @click="copyToClipboard(sub.m3u8Url)"
                  >
                    <CommonIcon name="copy" class-name="h-3.5 w-3.5" />
                  </button>
                  <button
                    v-if="isResumableStatus(sub.status)"
                    class="btn btn-ghost btn-circle btn-sm text-success h-7 w-7"
                    @click="store.resumeTask(sub.id)"
                  >
                    <CommonIcon name="play" class-name="h-3.5 w-3.5" />
                  </button>
                  <button
                    v-else-if="isPausableSubtaskStatus(sub.status)"
                    class="btn btn-ghost btn-circle btn-sm text-warning h-7 w-7"
                    @click="store.pauseTask(sub.id)"
                  >
                    <CommonIcon name="pause" class-name="h-3.5 w-3.5" />
                  </button>
                  <button
                    class="btn btn-ghost btn-circle btn-sm text-error h-7 w-7"
                    @click="store.deleteTask(sub.id)"
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
emplate>
late>
