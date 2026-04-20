import axios from 'axios';
import type {
  AddTaskPayload,
  AppSettings,
  AppVersionInfo,
  FilesResponse,
  FolderInfo,
  TaskCategory,
  TaskGroup,
  TaskItem,
  TaskStatus,
} from '../types/app';

const TASK_STATUSES: TaskStatus[] = [
  'pending',
  'parsing',
  'downloading',
  'merging',
  'completed',
  'failed',
  'paused',
  'skipped',
  'active',
];

const TASK_CATEGORIES: TaskCategory[] = ['movie', 'series', 'other'];

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null;

const asString = (value: unknown, fallback = '') => (typeof value === 'string' ? value : fallback);
const asNullableString = (value: unknown) => (typeof value === 'string' ? value : null);
const asBoolean = (value: unknown, fallback = false) =>
  typeof value === 'boolean' ? value : fallback;
const asNumber = (value: unknown, fallback = 0) =>
  typeof value === 'number' && Number.isFinite(value) ? value : fallback;
const asNullableNumber = (value: unknown) =>
  typeof value === 'number' && Number.isFinite(value) ? value : null;
const asTaskStatus = (value: unknown): TaskStatus =>
  typeof value === 'string' && TASK_STATUSES.includes(value as TaskStatus)
    ? (value as TaskStatus)
    : 'pending';
const asTaskCategory = (value: unknown): TaskCategory =>
  typeof value === 'string' && TASK_CATEGORIES.includes(value as TaskCategory)
    ? (value as TaskCategory)
    : 'other';

const parseTaskItem = (value: unknown): TaskItem | null => {
  if (!isRecord(value)) return null;

  const id = asString(value.id);
  const title = asString(value.title);
  if (!id || !title) return null;

  return {
    id,
    parentId: asNullableString(value.parentId),
    groupTitle: asNullableString(value.groupTitle),
    title,
    type: asTaskCategory(value.type),
    year: asNullableString(value.year),
    season: asNullableString(value.season),
    m3u8Url: asNullableString(value.m3u8Url),
    status: asTaskStatus(value.status),
    isPendingOverwrite: asBoolean(value.isPendingOverwrite),
    percentage: asNumber(value.percentage),
    totalSegments: asNumber(value.totalSegments),
    completedSegments: asNumber(value.completedSegments),
    estimatedSize: asNullableNumber(value.estimatedSize),
    outputPath: asNullableString(value.outputPath),
    createdAt: asString(value.createdAt),
    updatedAt: asString(value.updatedAt),
  };
};

const parseTaskGroup = (value: unknown): TaskGroup | null => {
  const task = parseTaskItem(value);
  if (!task || !isRecord(value)) return null;

  const subtasks = Array.isArray(value.subtasks)
    ? value.subtasks.map(parseTaskItem).filter((item): item is TaskItem => item !== null)
    : [];

  return { ...task, subtasks };
};

const parseFolderInfo = (value: unknown): FolderInfo | null => {
  if (!isRecord(value)) return null;

  const id = asString(value.id);
  const name = asString(value.name);
  if (!id || !name) return null;

  const folders = Array.isArray(value.folders)
    ? value.folders.map(parseFolderInfo).filter((item): item is FolderInfo => item !== null)
    : [];

  const files = Array.isArray(value.files)
    ? value.files
        .map((file) => {
          if (!isRecord(file)) return null;
          const fileId = asString(file.id);
          const fileName = asString(file.name);
          if (!fileId || !fileName) return null;

          return {
            id: fileId,
            name: fileName,
            size: asString(file.size),
            updatedAt: asString(file.updatedAt),
          };
        })
        .filter((file): file is FolderInfo['files'][number] => file !== null)
    : [];

  return {
    id,
    name,
    fileCount: asNumber(value.fileCount),
    updatedAt: asString(value.updatedAt),
    folders,
    files,
  };
};

const parseTasksResponse = (value: unknown): TaskGroup[] =>
  Array.isArray(value)
    ? value.map(parseTaskGroup).filter((item): item is TaskGroup => item !== null)
    : [];

const parseFilesResponse = (value: unknown): FilesResponse => {
  if (!isRecord(value)) {
    return { folders: [], downloadPath: '' };
  }

  return {
    folders: Array.isArray(value.folders)
      ? value.folders.map(parseFolderInfo).filter((item): item is FolderInfo => item !== null)
      : [],
    downloadPath: asString(value.downloadPath),
  };
};

const parseSettingsResponse = (value: unknown): Partial<AppSettings> => {
  if (!isRecord(value)) return {};

  return {
    concurrency: asString(value.concurrency),
    retryCount: asString(value.retryCount),
    retryDelay: asString(value.retryDelay),
    userAgent: asString(value.userAgent),
    proxy: asString(value.proxy),
  };
};

const parseVersionResponse = (value: unknown): AppVersionInfo => {
  if (!isRecord(value)) {
    return {
      appVersion: 'unknown',
      dockerImage: 'ghcr.io/hpyer/m3u8-harvester',
      dockerVersion: '1.0.1',
      tauriVersion: null,
    };
  }

  return {
    appVersion: asString(value.appVersion, 'unknown'),
    dockerImage: asString(value.dockerImage, 'ghcr.io/hpyer/m3u8-harvester'),
    dockerVersion: asString(value.dockerVersion, '1.0.1'),
    tauriVersion: asNullableString(value.tauriVersion),
  };
};

export interface AppApi {
  getTasks(): Promise<TaskGroup[]>;
  respondOverwrite(taskId: string, overwrite: boolean): Promise<void>;
  pauseTask(id: string): Promise<void>;
  resumeTask(id: string): Promise<void>;
  deleteTask(id: string): Promise<void>;
  getFiles(): Promise<FilesResponse>;
  getSettings(): Promise<Partial<AppSettings>>;
  getVersionInfo(): Promise<AppVersionInfo>;
  saveSettings(settings: AppSettings): Promise<void>;
  createTask(task: AddTaskPayload): Promise<void>;
  deleteFile(id: string): Promise<void>;
  deleteFolder(id: string): Promise<void>;
  renameFileOrFolder(id: string, newName: string): Promise<void>;
}

const API_BASE =
  import.meta.env.VITE_API_BASE_URL ||
  (window.location.port === '5173' ? 'http://localhost:6868' : window.location.origin);

class HttpAppApi implements AppApi {
  async getTasks() {
    const res = await axios.get<unknown>(`${API_BASE}/api/tasks`);
    return parseTasksResponse(res.data);
  }

  async respondOverwrite(taskId: string, overwrite: boolean) {
    await axios.post(`${API_BASE}/api/tasks/${taskId}/overwrite`, { overwrite });
  }

  async pauseTask(id: string) {
    await axios.post(`${API_BASE}/api/tasks/${id}/pause`);
  }

  async resumeTask(id: string) {
    await axios.post(`${API_BASE}/api/tasks/${id}/resume`);
  }

  async deleteTask(id: string) {
    await axios.delete(`${API_BASE}/api/tasks/${id}`);
  }

  async getFiles() {
    const res = await axios.get<unknown>(`${API_BASE}/api/files`);
    return parseFilesResponse(res.data);
  }

  async getSettings() {
    const res = await axios.get<unknown>(`${API_BASE}/api/settings`);
    return parseSettingsResponse(res.data);
  }

  async getVersionInfo() {
    const res = await axios.get<unknown>(`${API_BASE}/api/meta/version`);
    return parseVersionResponse(res.data);
  }

  async saveSettings(settings: AppSettings) {
    await axios.post(`${API_BASE}/api/settings`, settings);
  }

  async createTask(task: AddTaskPayload) {
    await axios.post(`${API_BASE}/api/tasks`, task);
  }

  async deleteFile(id: string) {
    await axios.delete(`${API_BASE}/api/files/${encodeURIComponent(id)}`);
  }

  async deleteFolder(id: string) {
    await axios.delete(`${API_BASE}/api/files/folders/${encodeURIComponent(id)}`);
  }

  async renameFileOrFolder(id: string, newName: string) {
    await axios.post(`${API_BASE}/api/files/${encodeURIComponent(id)}/rename`, { newName });
  }
}

let apiClient: AppApi = new HttpAppApi();

export const api = {
  getTasks: () => apiClient.getTasks(),
  respondOverwrite: (taskId: string, overwrite: boolean) =>
    apiClient.respondOverwrite(taskId, overwrite),
  pauseTask: (id: string) => apiClient.pauseTask(id),
  resumeTask: (id: string) => apiClient.resumeTask(id),
  deleteTask: (id: string) => apiClient.deleteTask(id),
  getFiles: () => apiClient.getFiles(),
  getSettings: () => apiClient.getSettings(),
  getVersionInfo: () => apiClient.getVersionInfo(),
  saveSettings: (settings: AppSettings) => apiClient.saveSettings(settings),
  createTask: (task: AddTaskPayload) => apiClient.createTask(task),
  deleteFile: (id: string) => apiClient.deleteFile(id),
  deleteFolder: (id: string) => apiClient.deleteFolder(id),
  renameFileOrFolder: (id: string, newName: string) => apiClient.renameFileOrFolder(id, newName),
};

export const setApiClient = (client: AppApi) => {
  apiClient = client;
};
