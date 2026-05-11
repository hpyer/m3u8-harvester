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

const openTmdbSearchModal = async () => {
  store.openTmdbSearchModal();
  if (isTmdbConfigured.value) {
    await store.searchTmdb();
  }
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
    <div class="modal-box w-11/12 max-w-3xl p-0 flex flex-col max-h-[88vh] overflow-hidden">
      <div class="px-5 py-3 border-b border-base-300 bg-base-100 shrink-0">
        <h3 class="font-bold text-base">添加新任务</h3>
      </div>

      <div class="flex-1 overflow-y-auto px-5 py-4">
        <div class="flex flex-col gap-3">
          <div class="form-control">
            <label class="label py-1">
              <span class="label-text font-bold">任务名称 (目录名)</span>
            </label>
            <div class="join w-full">
              <input
                v-model="store.addTaskData.title"
                type="text"
                placeholder="例如：奥本海默"
                class="input input-bordered input-sm join-item w-full"
              />
              <button
                type="button"
                class="btn btn-primary btn-sm join-item"
                :disabled="store.tmdbSearchLoading || !store.addTaskData.title.trim()"
                @click="openTmdbSearchModal"
              >
                {{ store.tmdbSearchLoading ? '查询中' : 'TMDB 查询' }}
              </button>
            </div>
            <p v-if="!isTmdbConfigured" class="text-xs text-warning mt-1">
              TMDB 未配置，仍可手动填写任务。
            </p>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text font-bold">内容分类</span>
            </label>
            <div class="join">
              <button
                type="button"
                class="btn btn-sm join-item"
                :class="store.addTaskData.category === 'movie' ? 'btn-primary' : 'btn-outline'"
                @click="setCategory('movie')"
              >
                电影
              </button>
              <button
                type="button"
                class="btn btn-sm join-item"
                :class="store.addTaskData.category === 'series' ? 'btn-primary' : 'btn-outline'"
                @click="setCategory('series')"
              >
                剧集/综艺/动漫
              </button>
              <button
                type="button"
                class="btn btn-sm join-item"
                :class="store.addTaskData.category === 'other' ? 'btn-primary' : 'btn-outline'"
                @click="setCategory('other')"
              >
                其它
              </button>
            </div>
          </div>

          <div v-if="store.addTaskData.category === 'movie'" class="form-control md:w-1/2">
            <label class="label py-1"><span class="label-text font-medium">年份</span></label>
            <input
              v-model="store.addTaskData.year"
              type="text"
              placeholder="例如：2023"
              class="input input-bordered input-sm w-full"
            />
          </div>

          <div
            v-if="store.addTaskData.category === 'series'"
            class="grid grid-cols-1 md:grid-cols-2 gap-3"
          >
            <div class="form-control">
              <label class="label py-1"><span class="label-text font-medium">季号</span></label>
              <input
                v-model="store.addTaskData.season"
                type="text"
                placeholder="例如：1 (特别季填 0)"
                class="input input-bordered input-sm w-full"
                @change="store.loadTmdbSeason"
                @input="store.updateTmdbEpisodeMapping"
              />
            </div>
            <div v-if="store.selectedTmdbResult?.mediaType === 'tv'" class="form-control">
              <label class="label py-1"><span class="label-text font-medium">起始集</span></label>
              <input
                v-model="store.tmdbStartEpisode"
                type="number"
                min="1"
                class="input input-bordered input-sm w-full"
                @input="store.updateTmdbEpisodeMapping"
              />
            </div>
          </div>

          <div class="form-control">
            <label class="label py-1">
              <span class="label-text font-bold">子任务列表 (m3u8 链接)</span>
              <span class="label-text-alt opacity-50 text-xs">自动提取所有 m3u8 链接</span>
            </label>
            <textarea
              v-model="store.addTaskData.rawSubtasks"
              class="textarea textarea-bordered h-24 font-mono text-xs"
              placeholder="粘贴包含 m3u8 链接的文本，支持多行或混合内容"
              @input="parseRows"
            ></textarea>
          </div>

          <div class="form-control">
            <div class="rounded-lg border border-base-300 overflow-hidden min-h-28">
              <div class="bg-base-200 px-3 py-1.5 text-sm font-semibold">
                任务预览
                <span v-if="store.namingRows.length" class="badge badge-neutral badge-sm ml-2">
                  {{ store.namingRows.length }}
                </span>
              </div>
              <div
                v-if="store.namingRows.length"
                class="divide-y divide-base-300 max-h-52 overflow-y-auto"
              >
                <div
                  v-for="row in store.namingRows"
                  :key="row.lineIndex"
                  class="grid grid-cols-1 md:grid-cols-[3rem_1fr_12rem] gap-2 p-2 items-center"
                >
                  <span class="badge badge-ghost badge-sm">#{{ row.lineIndex + 1 }}</span>
                  <div class="min-w-0">
                    <p class="truncate font-mono text-xs opacity-70">{{ row.url }}</p>
                    <p v-if="row.generatedTitle" class="text-xs mt-0.5">
                      建议：<span class="font-mono">{{ row.generatedTitle }}</span>
                      <span v-if="row.episodeName" class="opacity-60">
                        · {{ row.episodeName }}
                      </span>
                    </p>
                  </div>
                  <input
                    :value="row.manualTitle"
                    type="text"
                    class="input input-bordered input-sm w-full"
                    placeholder="文件标题"
                    @input="updateManualTitle(row.lineIndex, $event)"
                  />
                </div>
              </div>
              <div v-else class="px-3 py-6 text-center text-sm opacity-50">
                粘贴 m3u8 链接后在这里编辑文件标题。
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="px-5 py-3 border-t border-base-300 bg-base-100 flex justify-end gap-2 shrink-0">
        <button class="btn btn-ghost btn-sm" @click="store.isAddTaskModalOpen = false">取消</button>
        <button class="btn btn-primary btn-sm px-6" @click="submit">提交任务</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click="store.isAddTaskModalOpen = false">
      <button>close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': store.isTmdbSearchModalOpen }">
    <div class="modal-box w-11/12 max-w-2xl">
      <h3 class="font-bold text-lg mb-4">选择 TMDB 结果</h3>

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
          type="button"
          class="btn btn-primary join-item"
          :disabled="!isTmdbConfigured || store.tmdbSearchLoading"
          @click="searchTmdb"
        >
          {{ store.tmdbSearchLoading ? '查询中' : '查询' }}
        </button>
      </div>

      <p v-if="!isTmdbConfigured" class="text-xs text-warning mt-2">
        请先在设置中填写 TMDB API Key 和 API 地址。
      </p>
      <p v-if="store.tmdbSearchError" class="text-xs text-error mt-2">
        {{ store.tmdbSearchError }}
      </p>

      <div v-if="store.tmdbSearchResults.length" class="grid gap-2 mt-4 max-h-80 overflow-y-auto">
        <button
          v-for="result in store.tmdbSearchResults"
          :key="`${result.mediaType}-${result.id}`"
          type="button"
          class="btn justify-between"
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
      <div
        v-else-if="!store.tmdbSearchLoading && !store.tmdbSearchError"
        class="py-10 text-center text-sm opacity-50"
      >
        输入名称后查询 TMDB 结果。
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="store.closeTmdbSearchModal">取消</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click="store.closeTmdbSearchModal">
      <button>close</button>
    </form>
  </dialog>
</template>
