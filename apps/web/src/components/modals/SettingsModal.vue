<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { ref, watch } from 'vue';
import { useAppStore } from '../../stores/appStore';
import type { AppSettings } from '../../types/app';
import CommonIcon from '../ui/CommonIcon.vue';

import { api } from '../../services/api';

const store = useAppStore();
const { versionInfo } = storeToRefs(store);
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
    <div class="modal-box w-11/12 max-w-2xl p-0 flex flex-col max-h-[90vh] overflow-hidden">
      <!-- Fixed Header -->
      <div class="px-6 py-4 border-b border-base-300 bg-base-100 shrink-0">
        <h3 class="font-bold text-lg flex items-center gap-2">
          <CommonIcon name="settings" class-name="h-6 w-6" />
          应用设置
        </h3>
      </div>

      <!-- Scrollable Content -->
      <div class="flex-1 overflow-y-auto p-6 bg-base-50/30">
        <div class="flex flex-col gap-5">
          <section class="rounded-xl border border-base-300 bg-base-200/40 p-4">
            <div class="mb-4">
              <h4 class="font-semibold text-sm text-primary">下载配置</h4>
              <p class="text-xs opacity-60 mt-1">
                这些配置会保存到服务端数据库，影响下载执行行为。
              </p>
            </div>

            <div class="flex flex-col gap-4">
              <div v-if="isTauri" class="form-control">
                <label class="label pb-1"
                  ><span class="label-text font-medium">下载目录</span></label
                >
                <div class="join">
                  <input
                    v-model="localSettings.downloadPath"
                    type="text"
                    readonly
                    class="input input-bordered w-full join-item bg-base-100 text-sm"
                    placeholder="选择下载保存目录"
                  />
                  <button class="btn btn-primary join-item" @click="selectDirectory">
                    <CommonIcon name="folder" class-name="h-4 w-4" />
                    选择
                  </button>
                </div>
                <p class="text-[10px] mt-1.5 opacity-50 px-1 italic">
                  修改下载目录后，临时文件将存放在该目录下的 .temp 文件夹中。
                </p>
              </div>

              <div class="form-control">
                <label class="label pb-1"
                  ><span class="label-text font-medium">并发下载数</span></label
                >
                <input
                  v-model="localSettings.concurrency"
                  type="number"
                  min="1"
                  class="input input-bordered w-full"
                />
              </div>

              <div class="form-control">
                <label class="label pb-1"
                  ><span class="label-text font-medium">分片重试次数</span></label
                >
                <input
                  v-model="localSettings.retryCount"
                  type="number"
                  min="1"
                  class="input input-bordered w-full"
                />
              </div>

              <div class="form-control">
                <label class="label pb-1"
                  ><span class="label-text font-medium">重试间隔 (毫秒)</span></label
                >
                <input
                  v-model="localSettings.retryDelay"
                  type="number"
                  min="0"
                  class="input input-bordered w-full"
                />
              </div>

              <div class="form-control">
                <label class="label pb-1"
                  ><span class="label-text font-medium">User-Agent</span></label
                >
                <textarea
                  v-model="localSettings.userAgent"
                  rows="3"
                  class="textarea textarea-bordered w-full text-xs"
                />
              </div>

              <div class="form-control">
                <label class="label pb-1"
                  ><span class="label-text font-medium">代理服务器</span></label
                >
                <input
                  v-model="localSettings.proxy"
                  type="text"
                  placeholder="http://127.0.0.1:7890 或 socks5://127.0.0.1:7890"
                  class="input input-bordered w-full"
                />
              </div>
            </div>
          </section>

          <section class="rounded-xl border border-base-300 bg-base-100 p-4">
            <div class="mb-4">
              <h4 class="font-semibold text-sm text-primary">版本信息</h4>
              <p class="text-xs opacity-60 mt-1">构建元数据单独展示，不再混入下载设置。</p>
            </div>

            <div class="grid gap-2 text-sm">
              <div
                class="flex items-center justify-between gap-4 rounded-lg bg-base-200/60 px-3 py-2"
              >
                <span class="opacity-70">Docker 镜像</span>
                <span class="text-right font-mono text-xs"
                  >{{ versionInfo.dockerImage }}:{{ versionInfo.dockerVersion }}</span
                >
              </div>
              <div
                class="flex items-center justify-between gap-4 rounded-lg bg-base-200/60 px-3 py-2"
              >
                <span class="opacity-70">应用版本</span>
                <span class="font-mono text-xs">v{{ versionInfo.appVersion }}</span>
              </div>
              <div
                class="flex items-center justify-between gap-4 rounded-lg bg-base-200/60 px-3 py-2"
              >
                <span class="opacity-70">Tauri</span>
                <span class="font-mono text-xs">{{
                  versionInfo.tauriVersion ? `v${versionInfo.tauriVersion}` : '未接入'
                }}</span>
              </div>
            </div>
          </section>
        </div>
      </div>

      <!-- Fixed Footer -->
      <div class="px-6 py-4 border-t border-base-300 bg-base-100 flex justify-end gap-3 shrink-0">
        <button class="btn btn-ghost" @click="store.isSettingsModalOpen = false">取消</button>
        <button class="btn btn-primary px-8" @click="save">保存配置</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click="store.isSettingsModalOpen = false">
      <button>close</button>
    </form>
  </dialog>
</template>
