<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { useAppStore } from './stores/appStore';

// Components
import Header from './components/layout/Header.vue';
import Footer from './components/layout/Footer.vue';
import TaskList from './components/features/TaskList.vue';
import LocalFiles from './components/features/LocalFiles.vue';
import AddTaskModal from './components/modals/AddTaskModal.vue';
import SettingsModal from './components/modals/SettingsModal.vue';
import OverwriteModal from './components/modals/OverwriteModal.vue';
import CommonIcon from './components/ui/CommonIcon.vue';

const store = useAppStore();
const activeTab = ref('tasks');

watch(activeTab, (newTab) => {
  if (newTab === 'files') {
    store.fetchLocalFiles();
  }
});

onMounted(async () => {
  // 初始化主题
  document.documentElement.setAttribute('data-theme', store.theme);

  await store.loadSettings();

  store.fetchTasks();
});

onUnmounted(() => {});
</script>

<template>
  <div class="min-h-screen flex flex-col bg-base-200">
    <Header />

    <!-- Main Content -->
    <main class="flex-1 container mx-auto p-2 md:p-8">
      <!-- Tabs Navigation -->
      <div
        class="tabs tabs-boxed mb-3 md:mb-6 bg-base-100 p-1 flex justify-center md:justify-start shadow-sm border border-base-200"
      >
        <a
          class="tab tab-sm md:tab-lg flex gap-2 flex-1 md:flex-none transition-all duration-200"
          :class="{ 'tab-active font-bold': activeTab === 'tasks' }"
          @click="activeTab = 'tasks'"
        >
          <CommonIcon name="tasks" class-name="h-4 w-4" />
          任务列表
        </a>
        <a
          class="tab tab-sm md:tab-lg flex gap-2 flex-1 md:flex-none transition-all duration-200"
          :class="{ 'tab-active font-bold': activeTab === 'files' }"
          @click="activeTab = 'files'"
        >
          <CommonIcon name="files" class-name="h-4 w-4" />
          本地文件
        </a>
      </div>

      <!-- Tab Content Area -->
      <div class="bg-base-100 rounded-box shadow-sm border border-base-200 min-h-[400px]">
        <TaskList v-if="activeTab === 'tasks'" />
        <LocalFiles v-if="activeTab === 'files'" />
      </div>
    </main>

    <Footer />

    <!-- Modals -->
    <AddTaskModal />
    <SettingsModal />

    <OverwriteModal />
  </div>
</template>

<style>
html,
body {
  margin: 0;
  padding: 0;
}
</style>
