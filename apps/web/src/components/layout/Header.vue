<script setup lang="ts">
import { useAppStore } from '../../stores/appStore';
import CommonIcon from '../ui/CommonIcon.vue';
const store = useAppStore();
</script>

<template>
  <header class="navbar bg-base-100 shadow-sm px-2 md:px-8">
    <div class="flex-1">
      <a class="btn btn-ghost normal-case text-base md:text-xl gap-1 md:gap-2 px-2">
        <img src="/icon.png" alt="" class="h-7 w-7 md:h-8 md:w-8 rounded-md" />
        <span class="font-bold">M3U8 Harvester</span>
      </a>
    </div>
    <div class="flex-none gap-2">
      <button class="btn btn-ghost btn-circle" @click="store.isSettingsModalOpen = true">
        <CommonIcon name="settings" class-name="h-6 w-6" />
      </button>

      <label class="swap swap-rotate btn btn-ghost btn-circle">
        <input type="checkbox" :checked="store.theme === 'dark'" @change="store.toggleTheme()" />
        <CommonIcon name="sun" class-name="swap-on w-6 h-6" />
        <CommonIcon name="moon" class-name="swap-off w-6 h-6" />
      </label>
    </div>
  </header>
</template>
