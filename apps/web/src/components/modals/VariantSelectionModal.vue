<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';

const store = useAppStore();

const formatBandwidth = (bandwidth: number, averageBandwidth: number | null) => {
  const value = averageBandwidth ?? bandwidth;
  if (!value) return '-';
  return `${(value / 1000).toFixed(0)} kbps`;
};
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': store.isVariantSelectionModalOpen }">
    <div class="modal-box w-11/12 max-w-3xl">
      <div class="flex items-start justify-between gap-4 mb-5">
        <div>
          <h3 class="font-bold text-lg">选择清晰度</h3>
          <p class="text-sm opacity-70 mt-1">检测到多清晰度 M3U8，请确认要下载的版本。</p>
        </div>
        <span class="badge badge-primary badge-outline">{{
          store.variantSelectionItems.length
        }}</span>
      </div>

      <div class="flex flex-col gap-4 max-h-[60vh] overflow-y-auto">
        <section
          v-for="item in store.variantSelectionItems"
          :key="item.lineIndex"
          class="rounded-xl border border-base-300 p-4 bg-base-200/40"
        >
          <div class="mb-3">
            <div class="font-semibold break-all">
              {{ item.title || `第 ${item.lineIndex + 1} 条任务` }}
            </div>
            <div class="text-xs opacity-60 break-all mt-1">{{ item.url }}</div>
          </div>

          <div class="space-y-2">
            <label
              v-for="(variant, variantIndex) in item.probe.variants"
              :key="`${item.lineIndex}-${variantIndex}`"
              class="flex items-start gap-3 rounded-lg border border-base-300 px-3 py-2 cursor-pointer hover:bg-base-100"
            >
              <input
                class="radio radio-primary radio-sm mt-1"
                type="radio"
                :name="`variant-${item.lineIndex}`"
                :checked="item.selectedIndex === variantIndex"
                @change="store.updateVariantSelection(item.lineIndex, variantIndex)"
              />
              <div class="text-sm">
                <div class="font-medium">
                  {{ variant.resolution || '未知分辨率' }}
                  <span
                    v-if="variant.hasSeparateAudio"
                    class="badge badge-sm badge-outline font-semibold py-1.5 px-2 scale-90 ml-2 align-middle"
                  >
                    音视频分离
                  </span>
                </div>
                <div class="opacity-70 mt-1">
                  码率 {{ formatBandwidth(variant.bandwidth, variant.averageBandwidth) }}
                  <span v-if="variant.audioName"> · 音轨 {{ variant.audioName }}</span>
                </div>
                <div v-if="variant.codecs" class="opacity-50 text-xs mt-1 break-all">
                  {{ variant.codecs }}
                </div>
              </div>
            </label>
          </div>
        </section>
      </div>

      <div class="modal-action">
        <button class="btn btn-ghost" @click="store.cancelVariantSelections()">取消</button>
        <button class="btn btn-primary w-40" @click="store.confirmVariantSelections()">
          立即下载（{{ store.variantSelectionCountdown }}秒）
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @click.prevent="store.cancelVariantSelections()">
      <button>close</button>
    </form>
  </dialog>
</template>
