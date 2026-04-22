export type TaskStatus =
  | 'pending'
  | 'parsing'
  | 'downloading'
  | 'merging'
  | 'completed'
  | 'failed'
  | 'paused'
  | 'skipped'
  | 'active';

export type TaskCategory = 'movie' | 'series' | 'other';

export interface TaskItem {
  id: string;
  parentId: string | null;
  groupTitle: string | null;
  title: string;
  type: TaskCategory;
  year: string | null;
  season: string | null;
  m3u8Url: string | null;
  status: TaskStatus;
  isPendingOverwrite: boolean;
  percentage: number;
  totalSegments: number;
  completedSegments: number;
  estimatedSize: number | null;
  outputPath: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface TaskGroup extends TaskItem {
  subtasks: TaskItem[];
}

export interface FileInfo {
  id: string;
  name: string;
  size: string;
  updatedAt: string;
}

export interface FolderInfo {
  id: string;
  name: string;
  fileCount: number;
  updatedAt: string;
  folders: FolderInfo[];
  files: FileInfo[];
}

export interface FilesResponse {
  folders: FolderInfo[];
  downloadPath: string;
}

export interface AppSettings {
  concurrency: string;
  retryCount: string;
  retryDelay: string;
  userAgent: string;
  proxy: string;
  downloadPath?: string;
}

export interface AppVersionInfo {
  appVersion: string;
  dockerImage: string;
  dockerVersion: string;
  tauriVersion: string | null;
}

export interface AddTaskPayload {
  title: string;
  category: TaskCategory;
  year: string;
  season: string;
  rawSubtasks: string;
}

export interface ConfirmationItem {
  taskId: string;
  fileName: string;
}
