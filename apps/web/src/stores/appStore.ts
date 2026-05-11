import { defineStore } from 'pinia';
import { api } from '../services/api';
import type {
  AddTaskPayload,
  AppSettings,
  AppVersionInfo,
  ConfirmationItem,
  FolderInfo,
  M3U8NamingRow,
  M3U8StreamSelection,
  TaskCategory,
  TaskGroup,
  TaskItem,
  TaskStatus,
  TmdbSearchResult,
  TmdbSeasonDetails,
  VariantSelectionItem,
} from '../types/app';

type StatusSnapshot = Record<string, TaskStatus>;

const DEFAULT_SETTINGS: AppSettings = {
  concurrency: '5',
  retryCount: '3',
  retryDelay: '2000',
  userAgent:
    'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  proxy: '',
  tmdbApiKey: '',
  tmdbApiBaseUrl: 'https://api.themoviedb.org/3',
};

const DEFAULT_VERSION_INFO: AppVersionInfo = {
  appVersion: 'unknown',
  dockerImage: 'ghcr.io/hpyer/m3u8-harvester',
  dockerVersion: '1.1.0',
  tauriVersion: null,
};

const createEmptyAddTaskPayload = (): AddTaskPayload => ({
  title: '',
  category: 'movie',
  year: '',
  season: '',
  rawSubtasks: '',
  streamSelections: {},
});

const ACTIVE_STATUSES: TaskStatus[] = ['downloading', 'merging', 'pending', 'parsing', 'active'];
const RUNNABLE_STATUSES: TaskStatus[] = ['pending', 'downloading', 'parsing', 'merging', 'active'];
const RESUMABLE_STATUSES: TaskStatus[] = ['paused', 'failed'];
const M3U8_URL_RE = /https?:\/\/[^\s"'<>]+?\.m3u8(?:\?[^\s"'<>]*)?/gi;
let variantSelectionCountdownTimer: ReturnType<typeof setInterval> | null = null;
let variantSelectionResolver: ((value: Record<string, M3U8StreamSelection> | null) => void) | null =
  null;

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
    isVariantSelectionModalOpen: false,
    variantSelectionItems: [] as VariantSelectionItem[],
    variantSelectionCountdown: 30,
    tmdbSearchQuery: '',
    tmdbSearchResults: [] as TmdbSearchResult[],
    tmdbSearchLoading: false,
    tmdbSearchError: '',
    isTmdbSearchModalOpen: false,
    selectedTmdbResult: null as TmdbSearchResult | null,
    selectedTmdbSeason: null as TmdbSeasonDetails | null,
    tmdbStartEpisode: '1',
    namingRows: [] as M3U8NamingRow[],
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
      this.closeVariantSelectionModal();
      this.resetTmdbTaskHelper();
      if (data) {
        this.addTaskData = {
          title: data.title || '',
          category: (data.category as TaskCategory | undefined) || 'movie',
          year: data.year || '',
          season: data.season || '',
          rawSubtasks: '',
          streamSelections: {},
        };
      } else {
        this.addTaskData = createEmptyAddTaskPayload();
      }
      this.isAddTaskModalOpen = true;
    },
    async searchTmdb() {
      const query = this.tmdbSearchQuery.trim() || this.addTaskData.title.trim();
      if (!query) {
        this.tmdbSearchResults = [];
        this.tmdbSearchError = '';
        return;
      }

      if (!this.settings.tmdbApiKey.trim()) {
        this.tmdbSearchError = '请先在设置中填写 TMDB API Key';
        this.tmdbSearchResults = [];
        return;
      }

      this.tmdbSearchLoading = true;
      this.tmdbSearchError = '';
      try {
        this.tmdbSearchResults = await api.searchTmdb(query);
      } catch (_error) {
        this.tmdbSearchResults = [];
        this.tmdbSearchError = 'TMDB 查询失败，请检查 API Key 或 API 地址';
      } finally {
        this.tmdbSearchLoading = false;
      }
    },
    openTmdbSearchModal() {
      this.tmdbSearchQuery = this.addTaskData.title.trim();
      this.tmdbSearchResults = [];
      this.tmdbSearchError = '';
      this.isTmdbSearchModalOpen = true;
    },
    closeTmdbSearchModal() {
      this.isTmdbSearchModalOpen = false;
    },
    async selectTmdbResult(result: TmdbSearchResult) {
      this.selectedTmdbResult = result;
      this.isTmdbSearchModalOpen = false;
      this.addTaskData.title = result.title;
      this.addTaskData.category = result.mediaType === 'movie' ? 'movie' : 'series';
      this.addTaskData.year = result.year ?? '';

      if (result.mediaType === 'tv') {
        if (!this.addTaskData.season) {
          this.addTaskData.season = '1';
        }
        await this.loadTmdbSeason();
      } else {
        this.selectedTmdbSeason = null;
      }

      this.parseNamingRows(this.addTaskData.rawSubtasks);
    },
    async loadTmdbSeason() {
      if (!this.selectedTmdbResult || this.selectedTmdbResult.mediaType !== 'tv') {
        this.selectedTmdbSeason = null;
        return;
      }

      const seasonNumber = Number.parseInt(this.addTaskData.season || '1', 10);
      const safeSeason = Number.isFinite(seasonNumber) && seasonNumber >= 0 ? seasonNumber : 1;
      this.addTaskData.season = String(safeSeason);

      try {
        this.selectedTmdbSeason = await api.getTmdbTvSeason(this.selectedTmdbResult.id, safeSeason);
      } catch (_error) {
        this.selectedTmdbSeason = null;
        this.tmdbSearchError = '季集信息加载失败，仍可手动命名';
      }

      this.parseNamingRows(this.addTaskData.rawSubtasks);
    },
    updateTmdbEpisodeMapping() {
      this.parseNamingRows(this.addTaskData.rawSubtasks);
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
            tmdbApiKey: String(settings.tmdbApiKey ?? this.settings.tmdbApiKey),
            tmdbApiBaseUrl: String(settings.tmdbApiBaseUrl ?? this.settings.tmdbApiBaseUrl),
            downloadPath: settings.downloadPath
              ? String(settings.downloadPath)
              : this.settings.downloadPath,
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
        const payload: Record<string, string> = {
          concurrency: String(newSettings.concurrency ?? this.settings.concurrency),
          retryCount: String(newSettings.retryCount ?? this.settings.retryCount),
          retryDelay: String(newSettings.retryDelay ?? this.settings.retryDelay),
          userAgent: String(newSettings.userAgent ?? this.settings.userAgent),
          proxy: String(newSettings.proxy ?? this.settings.proxy),
          tmdbApiKey: String(newSettings.tmdbApiKey ?? this.settings.tmdbApiKey),
          tmdbApiBaseUrl: String(newSettings.tmdbApiBaseUrl ?? this.settings.tmdbApiBaseUrl),
        };

        if (newSettings.downloadPath) {
          payload.downloadPath = newSettings.downloadPath;
        }

        await api.saveSettings(payload);
        this.settings = { ...this.settings, ...payload };
        this.isSettingsModalOpen = false;
        // 保存设置后，如果是桌面版可能更新了下载目录，刷新一下文件列表
        await this.fetchLocalFiles();
      } catch (_e) {
        alert('保存设置失败');
      }
    },
    async submitNewTask(task: AddTaskPayload) {
      try {
        const finalTask = this.rebuildRawSubtasksFromNamingRows(task);
        const streamSelections = await this.resolveStreamSelections(finalTask.rawSubtasks);
        if (streamSelections === null) {
          return;
        }

        await api.createTask({ ...finalTask, streamSelections });
        this.isAddTaskModalOpen = false;
        this.addTaskData = createEmptyAddTaskPayload();
        this.resetTmdbTaskHelper();
        await this.fetchTasks();
      } catch (_e) {
        alert('提交任务失败');
      }
    },
    parseNamingRows(rawSubtasks: string) {
      const existingManualTitles = new Map(
        this.namingRows.map((row) => [row.url, row.manualTitle]),
      );
      const urls = Array.from(rawSubtasks.matchAll(M3U8_URL_RE), (match) => match[0]);

      this.namingRows = urls.map((url, lineIndex) => ({
        lineIndex,
        url,
        originalTitle: '',
        generatedTitle: this.generateRowTitle(lineIndex),
        manualTitle: existingManualTitles.get(url) ?? '',
        episodeNumber: this.getEpisodeNumberForRow(lineIndex),
        episodeName: this.getEpisodeNameForRow(lineIndex),
      }));
    },
    generateRowTitle(lineIndex: number) {
      if (this.addTaskData.category !== 'series') {
        return '';
      }

      const season = Number.parseInt(this.addTaskData.season || '1', 10);
      const safeSeason = Number.isFinite(season) ? season : 1;
      const episode = this.getEpisodeNumberForRow(lineIndex) ?? lineIndex + 1;
      return `S${String(safeSeason).padStart(2, '0')}E${String(episode).padStart(2, '0')}`;
    },
    getEpisodeNumberForRow(lineIndex: number) {
      if (this.addTaskData.category !== 'series') return null;
      const start = Number.parseInt(this.tmdbStartEpisode || '1', 10);
      const safeStart = Number.isFinite(start) && start > 0 ? start : 1;
      return safeStart + lineIndex;
    },
    getEpisodeNameForRow(lineIndex: number) {
      const episodeNumber = this.getEpisodeNumberForRow(lineIndex);
      if (!episodeNumber || !this.selectedTmdbSeason) return null;
      return (
        this.selectedTmdbSeason.episodes.find((episode) => episode.episodeNumber === episodeNumber)
          ?.name ?? null
      );
    },
    setNamingRowManualTitle(lineIndex: number, manualTitle: string) {
      const row = this.namingRows.find((item) => item.lineIndex === lineIndex);
      if (row) {
        row.manualTitle = manualTitle;
      }
    },
    rebuildRawSubtasksFromNamingRows(task: AddTaskPayload): AddTaskPayload {
      if (this.namingRows.length === 0) {
        return task;
      }

      const rawSubtasks = this.namingRows
        .map((row) => {
          const finalTitle = row.manualTitle.trim() || row.generatedTitle.trim();
          return finalTitle ? `${row.url} ${finalTitle}` : row.url;
        })
        .join('\n');

      return { ...task, rawSubtasks };
    },
    resetTmdbTaskHelper() {
      this.tmdbSearchQuery = '';
      this.tmdbSearchResults = [];
      this.tmdbSearchLoading = false;
      this.tmdbSearchError = '';
      this.isTmdbSearchModalOpen = false;
      this.selectedTmdbResult = null;
      this.selectedTmdbSeason = null;
      this.tmdbStartEpisode = '1';
      this.namingRows = [];
    },
    async resolveStreamSelections(rawSubtasks: string) {
      const lines = rawSubtasks
        .split('\n')
        .map((line) => line.trim())
        .filter(Boolean);

      const probeResults = await Promise.all(
        lines.map(async (line, lineIndex) => {
          const [url, ...titleParts] = line.split(/\s+/);
          if (!url) return null;

          try {
            const probe = await api.probeM3U8(url);
            if (!probe.isMaster || probe.variants.length <= 1) {
              return null;
            }

            return {
              lineIndex,
              rawLine: line,
              url,
              title: titleParts.join(' '),
              selectedIndex: probe.defaultVariantIndex ?? 0,
              probe,
            } satisfies VariantSelectionItem;
          } catch (_error) {
            return null;
          }
        }),
      );

      const items = probeResults.filter((item): item is VariantSelectionItem => item !== null);
      if (items.length === 0) {
        return {};
      }

      this.isVariantSelectionModalOpen = true;
      this.variantSelectionItems = items;
      this.variantSelectionCountdown = 30;

      if (variantSelectionCountdownTimer) {
        clearInterval(variantSelectionCountdownTimer);
      }

      return await new Promise<Record<string, M3U8StreamSelection> | null>((resolve) => {
        variantSelectionResolver = resolve;
        variantSelectionCountdownTimer = setInterval(() => {
          if (this.variantSelectionCountdown <= 1) {
            this.confirmVariantSelections();
            return;
          }
          this.variantSelectionCountdown -= 1;
        }, 1000);
      });
    },
    updateVariantSelection(lineIndex: number, selectedIndex: number) {
      const item = this.variantSelectionItems.find((entry) => entry.lineIndex === lineIndex);
      if (item) {
        item.selectedIndex = selectedIndex;
      }
    },
    confirmVariantSelections() {
      const resolver = variantSelectionResolver;
      this.stopVariantSelectionCountdown();
      const selections = Object.fromEntries(
        this.variantSelectionItems.map((item) => {
          const variant = item.probe.variants[item.selectedIndex];
          return [
            String(item.lineIndex),
            {
              originalUrl: item.url,
              videoUrl: variant.videoUrl,
              audioUrl: variant.audioUrl,
              resolution: variant.resolution,
              bandwidth: variant.bandwidth,
              averageBandwidth: variant.averageBandwidth,
              codecs: variant.codecs,
              audioName: variant.audioName,
            } satisfies M3U8StreamSelection,
          ];
        }),
      );
      this.closeVariantSelectionModal();
      resolver?.(selections);
    },
    cancelVariantSelections() {
      const resolver = variantSelectionResolver;
      this.stopVariantSelectionCountdown();
      this.closeVariantSelectionModal();
      resolver?.(null);
    },
    closeVariantSelectionModal() {
      this.stopVariantSelectionCountdown();
      this.isVariantSelectionModalOpen = false;
      this.variantSelectionItems = [];
      this.variantSelectionCountdown = 30;
    },
    stopVariantSelectionCountdown() {
      if (variantSelectionCountdownTimer) {
        clearInterval(variantSelectionCountdownTimer);
        variantSelectionCountdownTimer = null;
      }
      variantSelectionResolver = null;
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
