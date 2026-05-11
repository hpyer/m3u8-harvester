<script setup lang="ts">
import { computed } from 'vue';
import { useAppStore } from '../../stores/appStore';
import type { TaskCategory, TmdbSearchResult } from '../../types/app';

const store = useAppStore();
const isTmdbConfigured = computed(() => Boolean(store.settings.tmdbApiKey.trim()));

const submit = async () => {
  await store.submitNewTask({ ...store.addTaskData });
};

const parseRows = () => {
  store.parseNamingRows(store.addTaskData.rawSubtasks);
};

const searchTmdb = async () => {
  await store.searchTmdb();
};

const selectTmdbResult = async (result: TmdbSearchResult) => {
  await store.selectTmdbResult(result);
};

const setCategory = (category: TaskCategory) => {
  store.addTaskData.category = category;
  parseRows();
};

const updateManualTitle = (lineIndex: number, event: Event) => {
  const target = event.target;
  if (target instanceof HTMLInputElement) {
    store.setNamingRowManualTitle(lineIndex, target.value);
  }
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': store.isAddTaskModalOpen }">
    <div class="modal-box w-11/12 max-w-4xl">
      <h3 class="font-bold text-lg mb-6">添加新任务</h3>

      <section class="mb-5 rounded-lg border border-base-300 bg-base-200/40 p-4">
        <div class="flex flex-col gap-3">
          <div class="flex items-center justify-between gap-3">
            <div>
              <h4 class="font-semibold text-sm">TMDB 辅助填表</h4>
              <p class="text-xs opacity-60 mt-1">可选，用于匹配电影/剧集信息并生成命名预览。</p>
            </div>
            <span v-if="!isTmdbConfigured" class="badge badge-warning badge-sm">未配置</span>
          </div>

          <div class="join w-full">
            <input
              v-model="store.tmdbSearchQuery"
              type="text"
              class="input input-bordered join-item w-full"
              :disabled="!isTmdbConfigured"
              placeholder="输入电影或剧集名称"
              @keyup.enter="searchTmdb"
            />
            <button
              class="btn btn-primary join-item"
              :disabled="!isTmdbConfigured || store.tmdbSearchLoading"
              @click="searchTmdb"
            >
              {{ store.tmdbSearchLoading ? '查询中' : '查询' }}
            </button>
          </div>

          <p v-if="!isTmdbConfigured" class="text-xs text-warning">
            请先在设置中填写 TMDB API Key 和 API 地址。
          </p>
          <p v-if="store.tmdbSearchError" class="text-xs text-error">
            {{ store.tmdbSearchError }}
          </p>

          <div v-if="store.tmdbSearchResults.length" class="grid gap-2 max-h-40 overflow-y-auto">
            <button
              v-for="result in store.tmdbSearchResults"
              :key="`${result.mediaType}-${result.id}`"
              type="button"
              class="btn btn-sm justify-between"
              :class="
                store.selectedTmdbResult?.id === result.id &&
                store.selectedTmdbResult?.mediaType === result.mediaType
                  ? 'btn-primary'
                  : 'btn-outline'
              "
              @click="selectTmdbResult(result)"
            >
              <span class="truncate">{{ result.title }}</span>
              <span class="opacity-70">
                {{ result.mediaType === 'movie' ? '电影' : '剧集' }} {{ result.year || '' }}
              </span>
            </button>
          </div>
        </div>
      </section>

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
              @input="store.updateTmdbEpisodeMapping"
            />
          </div>
        </div>
      </div>

      <div
        v-if="
          store.addTaskData.category === 'series' && store.selectedTmdbResult?.mediaType === 'tv'
        "
        class="mt-4 grid grid-cols-1 md:grid-cols-3 gap-3"
      >
        <div class="form-control">
          <label class="label pb-1"><span class="label-text font-medium">TMDB 季号</span></label>
          <input
            v-model="store.tmdbSeasonNumber"
            type="number"
            min="0"
            class="input input-bordered w-full"
            @change="store.loadTmdbSeason"
          />
        </div>
        <div class="form-control">
          <label class="label pb-1"><span class="label-text font-medium">起始集</span></label>
          <input
            v-model="store.tmdbStartEpisode"
            type="number"
            min="1"
            class="input input-bordered w-full"
            @input="store.updateTmdbEpisodeMapping"
          />
        </div>
        <div class="flex items-end">
          <button class="btn btn-outline w-full" @click="store.loadTmdbSeason">刷新季集</button>
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
          @input="parseRows"
        ></textarea>
      </div>

      <div
        v-if="store.namingRows.length"
        class="mt-4 rounded-lg border border-base-300 overflow-hidden"
      >
        <div class="bg-base-200 px-3 py-2 text-sm font-semibold">命名预览</div>
        <div class="divide-y divide-base-300">
          <div
            v-for="row in store.namingRows"
            :key="row.lineIndex"
            class="grid grid-cols-1 md:grid-cols-[4rem_1fr_1fr] gap-3 p-3 items-center"
          >
            <span class="badge badge-ghost">#{{ row.lineIndex + 1 }}</span>
            <div class="min-w-0">
              <p class="truncate font-mono text-xs opacity-70">{{ row.url }}</p>
              <p v-if="row.generatedTitle" class="text-xs mt-1">
                建议：<span class="font-mono">{{ row.generatedTitle }}</span>
                <span v-if="row.episodeName" class="opacity-60"> · {{ row.episodeName }}</span>
              </p>
            </div>
            <input
              :value="row.manualTitle"
              type="text"
              class="input input-bordered input-sm w-full"
              placeholder="手动标题，留空则用建议标题"
              @input="updateManualTitle(row.lineIndex, $event)"
            />
          </div>
        </div>
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
