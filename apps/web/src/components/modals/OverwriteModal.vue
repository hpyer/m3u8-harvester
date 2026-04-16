<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';
import CommonIcon from '../ui/CommonIcon.vue';

const store = useAppStore();
</script>

<template>
  <dialog
    id="overwrite_modal"
    class="modal modal-bottom sm:modal-middle"
    :class="{ 'modal-open': store.confirmations.length > 0 }"
  >
    <div v-if="store.confirmations.length > 0" class="modal-box shadow-2xl border border-base-300">
      <h3 class="font-bold text-lg flex items-center gap-2">
        <CommonIcon name="warning" class-name="h-6 w-6 text-warning" />
        文件已存在
      </h3>
      <p class="py-4">
        子任务
        <span class="font-mono text-primary font-bold break-all">{{
          store.confirmations[0].fileName
        }}</span>
        已存在，是否重新下载并覆盖？
      </p>
      <div class="modal-action">
        <button
          class="btn btn-outline"
          @click="store.respondOverwrite(store.confirmations[0].taskId, false)"
        >
          跳过
        </button>
        <button
          class="btn btn-primary"
          @click="store.respondOverwrite(store.confirmations[0].taskId, true)"
        >
          覆盖下载
        </button>
      </div>
    </div>
    <div class="modal-backdrop bg-black/40 backdrop-blur-sm"></div>
  </dialog>
</template>
