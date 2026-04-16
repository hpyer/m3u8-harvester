<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';
import type { TaskCategory } from '../../types/app';

const store = useAppStore();

const submit = async () => {
  await store.submitNewTask({ ...store.addTaskData });
};

const setCategory = (category: TaskCategory) => {
  store.addTaskData.category = category;
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': store.isAddTaskModalOpen }">
    <div class="modal-box w-11/12 max-w-2xl">
      <h3 class="font-bold text-lg mb-6">添加新任务</h3>

      <div class="form-control mb-4">
        <label class="label">
          <span class="label-text font-bold">任务名称 (目录名)</span>
        </label>
        <input
          v-model="store.addTaskData.title"
          type="text"
          placeholder="例如：奥本海默"
          class="input input-bordered w-full"
        />
      </div>

      <div class="form-control">
        <label class="label">
          <span class="label-text font-bold">内容分类</span>
        </label>
        <div class="flex gap-4">
          <div class="join">
            <button
              type="button"
              class="btn join-item"
              :class="store.addTaskData.category === 'movie' ? 'btn-primary' : 'btn-outline'"
              @click="setCategory('movie')"
            >
              电影
            </button>
            <button
              type="button"
              class="btn join-item"
              :class="store.addTaskData.category === 'series' ? 'btn-primary' : 'btn-outline'"
              @click="setCategory('series')"
            >
              剧集/综艺/动漫
            </button>
            <button
              type="button"
              class="btn join-item"
              :class="store.addTaskData.category === 'other' ? 'btn-primary' : 'btn-outline'"
              @click="setCategory('other')"
            >
              其它
            </button>
          </div>

          <div v-if="store.addTaskData.category === 'movie'" class="flex-1">
            <input
              v-model="store.addTaskData.year"
              type="text"
              placeholder="年份，例如：2023"
              class="input input-bordered w-full"
            />
          </div>
          <div v-if="store.addTaskData.category === 'series'" class="flex-1">
            <input
              v-model="store.addTaskData.season"
              type="text"
              placeholder="季号，例如：1 (特别季填 0)"
              class="input input-bordered w-full"
            />
          </div>
        </div>
      </div>

      <div class="form-control mt-4">
        <label class="label">
          <span class="label-text font-bold">子任务列表 (m3u8 链接)</span>
          <span class="label-text-alt opacity-50 text-xs">每行一个：链接 [空格] 自定义文件名</span>
        </label>
        <textarea
          v-model="store.addTaskData.rawSubtasks"
          class="textarea textarea-bordered h-48 font-mono text-sm"
          placeholder="https://example.com/a.m3u8 第01集&#10;https://example.com/b.m3u8 第02集"
        ></textarea>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="store.isAddTaskModalOpen = false">取消</button>
        <button class="btn btn-primary px-8" @click="submit">提交任务</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click="store.isAddTaskModalOpen = false">
      <button>close</button>
    </form>
  </dialog>
</template>
