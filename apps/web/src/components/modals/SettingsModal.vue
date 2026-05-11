<script setup lang="ts">
import { ref, watch } from 'vue';
import { useAppStore } from '../../stores/appStore';
import type { AppSettings } from '../../types/app';
import CommonIcon from '../ui/CommonIcon.vue';

import { api } from '../../services/api';

const store = useAppStore();
const localSettings = ref<AppSettings>({ ...store.settings });
const isTauri = !!(window as any).__TAURI_INTERNALS__;

// 当 store 的设置加载完成时，同步到本地
watch(
  () => store.settings,
  (newVal) => {
    localSettings.value = { ...newVal };
  },
  { deep: true },
);

const selectDirectory = async () => {
  const path = await api.openSelectDirectory();
  if (path) {
    localSettings.value.downloadPath = path;
  }
};

const save = async () => {
  await store.saveSettings(localSettings.value);
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': store.isSettingsModalOpen }">
    <div class="modal-box w-11/12 max-w-2xl p-0 flex flex-col max-h-[88vh] overflow-hidden">
      <!-- Fixed Header -->
      <div class="px-5 py-3 border-b border-base-300 bg-base-100 shrink-0">
        <h3 class="font-bold text-base">应用设置</h3>
      </div>

      <!-- Scrollable Content -->
      <div class="flex-1 overflow-y-auto px-5 py-4 bg-base-50/30">
        <div class="flex flex-col gap-3">
          <section class="rounded-xl border border-base-300 bg-base-200/40 p-3">
            <div class="mb-3">
              <h4 class="font-semibold text-xs text-primary">下载配置</h4>
            </div>

            <div class="flex flex-col gap-3">
              <div v-if="isTauri" class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">下载目录</span></label
                >
                <div class="join">
                  <input
                    v-model="localSettings.downloadPath"
                    type="text"
                    readonly
                    class="input input-bordered input-sm w-full join-item bg-base-100"
                    placeholder="选择下载保存目录"
                  />
                  <button class="btn btn-primary btn-sm join-item" @click="selectDirectory">
                    <CommonIcon name="folder" class-name="h-3.5 w-3.5" />
                    选择
                  </button>
                </div>
              </div>

              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">并发下载数</span></label
                >
                <input
                  v-model="localSettings.concurrency"
                  type="number"
                  min="1"
                  class="input input-bordered input-sm w-full"
                />
              </div>

              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">分片重试次数</span></label
                >
                <input
                  v-model="localSettings.retryCount"
                  type="number"
                  min="1"
                  class="input input-bordered input-sm w-full"
                />
              </div>

              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">重试间隔 (毫秒)</span></label
                >
                <input
                  v-model="localSettings.retryDelay"
                  type="number"
                  min="0"
                  class="input input-bordered input-sm w-full"
                />
              </div>

              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">User-Agent</span></label
                >
                <textarea
                  v-model="localSettings.userAgent"
                  rows="2"
                  class="textarea textarea-bordered w-full text-xs"
                />
              </div>

              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">代理服务器</span></label
                >
                <input
                  v-model="localSettings.proxy"
                  type="text"
                  placeholder="http://127.0.0.1:7890 或 socks5://127.0.0.1:7890"
                  class="input input-bordered input-sm w-full"
                />
              </div>
            </div>
          </section>

          <section class="rounded-xl border border-base-300 bg-base-100 p-3">
            <div class="mb-3">
              <h4 class="font-semibold text-xs text-primary">TMDB 配置</h4>
            </div>

            <div class="flex flex-col gap-3">
              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">TMDB API Key</span></label
                >
                <input
                  v-model="localSettings.tmdbApiKey"
                  type="password"
                  autocomplete="off"
                  class="input input-bordered input-sm w-full"
                  placeholder="在 TMDB 账户设置中获取 API Key"
                />
              </div>

              <div class="form-control">
                <label class="label py-1"
                  ><span class="label-text font-medium text-xs">TMDB API 地址</span></label
                >
                <input
                  v-model="localSettings.tmdbApiBaseUrl"
                  type="text"
                  class="input input-bordered input-sm w-full"
                  placeholder="https://api.themoviedb.org/3"
                />
              </div>
            </div>
          </section>
        </div>
      </div>

      <!-- Fixed Footer -->
      <div class="px-5 py-3 border-t border-base-300 bg-base-100 flex justify-end gap-2 shrink-0">
        <button class="btn btn-ghost btn-sm" @click="store.isSettingsModalOpen = false">
          取消
        </button>
        <button class="btn btn-primary btn-sm px-6" @click="save">保存配置</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click="store.isSettingsModalOpen = false">
      <button>close</button>
    </form>
  </dialog>
</template>
