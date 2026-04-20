import { defineStore } from 'pinia';
import { api } from '../services/api';
import type {
  AddTaskPayload,
  AppSettings,
  AppVersionInfo,
  ConfirmationItem,
  FolderInfo,
  TaskCategory,
  TaskGroup,
  TaskItem,
  TaskStatus,
} from '../types/app';

type StatusSnapshot = Record<string, TaskStatus>;

const DEFAULT_SETTINGS: AppSettings = {
  concurrency: '5',
  retryCount: '3',
  retryDelay: '2000',
  userAgent:
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  proxy: '',
};

const DEFAULT_VERSION_INFO: AppVersionInfo = {
  appVersion: 'unknown',
  dockerImage: 'ghcr.io/hpyer/m3u8-harvester',
  dockerVersion: '1.0.1',
  tauriVersion: null,
};

const createEmptyAddTaskPayload = (): AddTaskPayload => ({
  title: '',
  category: 'movie',
  year: '',
  season: '',
  rawSubtasks: '',
});

const ACTIVE_STATUSES: TaskStatus[] = ['downloading', 'merging', 'pending', 'parsing', 'active'];
const RUNNABLE_STATUSES: TaskStatus[] = ['pending', 'downloading', 'parsing', 'merging', 'active'];
const RESUMABLE_STATUSES: TaskStatus[] = ['paused', 'failed'];

export const useAppStore = defineStore('app', {
  state: () => ({
    tasks: [] as TaskGroup[],
    localFolders: [] as FolderInfo[],
    downloadPath: '',
    settings: { ...DEFAULT_SETTINGS } as AppSettings,
    versionInfo: { ...DEFAULT_VERSION_INFO } as AppVersionInfo,
    isAddTaskModalOpen: false,
    addTaskData: createEmptyAddTaskPayload(),
    isSettingsModalOpen: false,
    theme: localStorage.getItem('theme') || 'cupcake',
    pollTimer: null as ReturnType<typeof setInterval> | null,
    confirmations: [] as ConfirmationItem[],
  }),
  actions: {
    async fetchTasks() {
      try {
        this.tasks = this.normalizeTasks(await api.getTasks());
        this.syncConfirmations();

        // 检查是否有活跃任务，决定是否开启自动刷新
        const hasActiveTasks = this.tasks.some(
          (parent) =>
            ACTIVE_STATUSES.includes(parent.status) ||
            parent.subtasks.some((sub) => ACTIVE_STATUSES.includes(sub.status)),
        );

        if (hasActiveTasks) {
          this.startPolling();
        } else {
          this.stopPolling();
        }
      } catch (_e) {
        console.error('Failed to fetch tasks', _e);
      }
    },
    syncConfirmations() {
      // 1. 找到所有数据库中标记为 isPendingOverwrite 的子任务
      const currentPendingInApi: ConfirmationItem[] = [];
      this.tasks.forEach((parent) => {
        parent.subtasks.forEach((sub) => {
          if (sub.isPendingOverwrite && sub.status === 'pending') {
            currentPendingInApi.push({
              taskId: sub.id,
              fileName: sub.title.endsWith('.mp4') ? sub.title : sub.title + '.mp4',
            });
          }
        });
      });

      // 2. 双向同步
      this.confirmations = currentPendingInApi;
    },
    startPolling() {
      if (this.pollTimer) return;
      this.pollTimer = setInterval(() => {
        api
          .getTasks()
          .then((tasks) => {
            this.tasks = this.normalizeTasks(tasks);
            this.syncConfirmations();
            const stillActive = this.tasks.some(
              (parent) =>
                ACTIVE_STATUSES.includes(parent.status) ||
                parent.subtasks.some((sub) => ACTIVE_STATUSES.includes(sub.status)),
            );

            if (!stillActive) this.stopPolling();
          })
          .catch(() => {});
      }, 3000);
    },
    stopPolling() {
      if (this.pollTimer) {
        clearInterval(this.pollTimer);
        this.pollTimer = null;
      }
    },
    updateTaskStatus(_data: unknown) {
      this.fetchTasks();
    },
    async respondOverwrite(taskId: string, overwrite: boolean) {
      try {
        await api.respondOverwrite(taskId, overwrite);
        this.confirmations = this.confirmations.filter((c) => c.taskId !== taskId);
        await this.fetchTasks(); // 确认后立即拉取最新状态
      } catch (_e) {
        alert('发送确认请求失败');
      }
    },
    async pauseTask(id: string) {
      // 乐观更新：立即寻找并更新本地状态
      const task = this.findTaskById(id);
      if (!task) return;

      const oldStatuses = this.captureStatuses(task);

      // 执行变更
      this.applyOptimisticPause(task);
      this.syncParentStatus(id);

      try {
        await api.pauseTask(id);
        // 等待一小会儿确保后端落库，然后强制拉取
        setTimeout(() => this.fetchTasks(), 600);
      } catch (_e) {
        this.rollbackStatuses(task, oldStatuses);
        this.syncParentStatus(id);
        alert('暂停失败');
      }
    },
    async resumeTask(id: string) {
      // 乐观更新
      const task = this.findTaskById(id);
      if (!task) return;

      const oldStatuses = this.captureStatuses(task);

      // 执行变更
      this.applyOptimisticResume(task);
      this.syncParentStatus(id);

      try {
        await api.resumeTask(id);
        setTimeout(() => this.fetchTasks(), 600);
      } catch (_e) {
        this.rollbackStatuses(task, oldStatuses);
        this.syncParentStatus(id);
        alert('恢复失败');
      }
    },
    applyOptimisticResume(task: TaskGroup | TaskItem) {
      if ('subtasks' in task) {
        task.status = 'active';
        task.subtasks.forEach((sub) => {
          if (RESUMABLE_STATUSES.includes(sub.status)) {
            sub.status = 'pending';
          }
        });
        return;
      }

      if (RESUMABLE_STATUSES.includes(task.status)) {
        task.status = 'pending';
      }
    },
    applyOptimisticPause(task: TaskGroup | TaskItem) {
      if ('subtasks' in task) {
        task.status = 'paused';
        task.subtasks.forEach((sub) => {
          if (RUNNABLE_STATUSES.includes(sub.status)) {
            sub.status = 'paused';
          }
        });
        return;
      }

      if (RUNNABLE_STATUSES.includes(task.status)) {
        task.status = 'paused';
      }
    },
    normalizeTasks(tasks: TaskGroup[]) {
      return tasks.map((parent) => {
        if (!parent.subtasks.length) return parent;

        const statuses = parent.subtasks.map((s) => s.status);
        if (statuses.every((s) => ['completed', 'skipped'].includes(s))) {
          parent.status = 'completed';
        } else if (statuses.every((s) => s === 'paused')) {
          parent.status = 'paused';
        } else if (statuses.some((s) => ACTIVE_STATUSES.includes(s))) {
          parent.status = 'active';
        } else if (
          statuses.some((s) => s === 'failed') &&
          !statuses.some((s) => ACTIVE_STATUSES.includes(s))
        ) {
          parent.status = 'failed';
        }

        return parent;
      });
    },
    // 递归更新任务及其子任务的状态
    updateTaskStatusRecursive(task: TaskGroup | TaskItem, status: TaskStatus) {
      task.status = status;
      if ('subtasks' in task) {
        task.subtasks.forEach((sub) => {
          sub.status = status;
        });
      }
    },
    // 捕获当前状态快照用于回滚
    captureStatuses(task: TaskGroup | TaskItem): StatusSnapshot {
      const statuses: StatusSnapshot = { [task.id]: task.status };
      if ('subtasks' in task) {
        task.subtasks.forEach((sub) => {
          statuses[sub.id] = sub.status;
        });
      }
      return statuses;
    },
    // 回滚状态
    rollbackStatuses(task: TaskGroup | TaskItem, oldStatuses: StatusSnapshot) {
      if (oldStatuses[task.id]) task.status = oldStatuses[task.id];
      if ('subtasks' in task) {
        task.subtasks.forEach((sub) => {
          if (oldStatuses[sub.id]) sub.status = oldStatuses[sub.id];
        });
      }
    },
    syncParentStatus(taskId: string) {
      // 如果更新的是子任务，尝试同步父任务状态
      for (const parent of this.tasks) {
        if (parent.subtasks.some((s) => s.id === taskId)) {
          const allCompleted = parent.subtasks.every((s) =>
            ['completed', 'skipped'].includes(s.status),
          );
          const allPaused = parent.subtasks.every((s) => s.status === 'paused');
          const anyFailed = parent.subtasks.some((s) => s.status === 'failed');
          const anyActive = parent.subtasks.some((s) => ACTIVE_STATUSES.includes(s.status));

          if (allCompleted) parent.status = 'completed';
          else if (allPaused) parent.status = 'paused';
          else if (anyActive) parent.status = 'active';
          else if (anyFailed) parent.status = 'failed';
          break;
        }
      }
    },
    findTaskById(id: string): TaskGroup | TaskItem | null {
      for (const parent of this.tasks) {
        if (parent.id === id) return parent;
        const sub = parent.subtasks.find((s) => s.id === id);
        if (sub) return sub;
      }
      return null;
    },
    async deleteTask(id: string) {
      if (!confirm('确定删除该下载任务吗？')) return;
      try {
        await api.deleteTask(id);
        await this.fetchTasks();
      } catch (_e) {
        alert('删除任务失败');
      }
    },
    openAddTaskModal(data?: Partial<AddTaskPayload>) {
      if (data) {
        this.addTaskData = {
          title: data.title || '',
          category: (data.category as TaskCategory | undefined) || 'movie',
          year: data.year || '',
          season: data.season || '',
          rawSubtasks: '',
        };
      } else {
        this.addTaskData = createEmptyAddTaskPayload();
      }
      this.isAddTaskModalOpen = true;
    },
    async fetchLocalFiles() {
      try {
        const res = await api.getFiles();
        this.localFolders = res.folders;
        this.downloadPath = res.downloadPath;
      } catch (_e) {
        console.error('Failed to fetch files', _e);
      }
    },
    async loadSettings() {
      try {
        const settings = await api.getSettings();
        if (Object.keys(settings).length > 0) {
          this.settings = {
            ...this.settings,
            concurrency: String(settings.concurrency ?? this.settings.concurrency),
            retryCount: String(settings.retryCount ?? this.settings.retryCount),
            retryDelay: String(settings.retryDelay ?? this.settings.retryDelay),
            userAgent: String(settings.userAgent ?? this.settings.userAgent),
            proxy: String(settings.proxy ?? this.settings.proxy),
          };
        }
      } catch (_e) {
        // Ignore settings load error
      }
    },
    async loadVersionInfo() {
      try {
        this.versionInfo = await api.getVersionInfo();
      } catch (_e) {
        // Ignore version info load error
      }
    },
    async saveSettings(newSettings: AppSettings) {
      try {
        const payload = {
          concurrency: String(newSettings.concurrency ?? this.settings.concurrency),
          retryCount: String(newSettings.retryCount ?? this.settings.retryCount),
          retryDelay: String(newSettings.retryDelay ?? this.settings.retryDelay),
          userAgent: String(newSettings.userAgent ?? this.settings.userAgent),
          proxy: String(newSettings.proxy ?? this.settings.proxy),
        };
        await api.saveSettings(payload);
        this.settings = { ...this.settings, ...payload };
        this.isSettingsModalOpen = false;
      } catch (_e) {
        alert('保存设置失败');
      }
    },
    async submitNewTask(task: AddTaskPayload) {
      try {
        await api.createTask(task);
        this.isAddTaskModalOpen = false;
        await this.fetchTasks();
      } catch (_e) {
        alert('提交任务失败');
      }
    },
    async deleteFile(id: string) {
      if (!confirm('确定删除该视频文件吗？')) return;
      try {
        await api.deleteFile(id);
        await this.fetchLocalFiles();
      } catch (_e) {
        alert('删除文件失败');
      }
    },
    async deleteFolder(id: string) {
      if (!confirm('确定删除该文件夹及其所有内容吗？')) return;
      try {
        await api.deleteFolder(id);
        await this.fetchLocalFiles();
      } catch (_e) {
        alert('删除文件夹失败');
      }
    },
    async renameFileOrFolder(id: string, newName: string) {
      try {
        await api.renameFileOrFolder(id, newName);
        await this.fetchLocalFiles();
      } catch (_e) {
        alert('重命名失败');
      }
    },
    toggleTheme() {
      this.theme = this.theme === 'dark' ? 'light' : 'dark';
      document.documentElement.setAttribute('data-theme', this.theme);
      localStorage.setItem('theme', this.theme);
    },
    setTheme(newTheme: string) {
      this.theme = newTheme;
      document.documentElement.setAttribute('data-theme', newTheme);
      localStorage.setItem('theme', newTheme);
    },
  },
});
