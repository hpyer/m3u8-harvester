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
  errorMessage: string | null;
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
  tmdbApiKey: string;
  tmdbApiBaseUrl: string;
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
  streamSelections?: Record<string, M3U8StreamSelection>;
}

export interface ConfirmationItem {
  taskId: string;
  fileName: string;
}

export interface M3U8VariantOption {
  videoUrl: string;
  audioUrl: string | null;
  resolution: string | null;
  bandwidth: number;
  averageBandwidth: number | null;
  codecs: string | null;
  audioName: string | null;
  hasSeparateAudio: boolean;
}

export interface M3U8ProbeResult {
  isMaster: boolean;
  defaultVariantIndex: number | null;
  variants: M3U8VariantOption[];
}

export interface M3U8StreamSelection {
  originalUrl: string;
  videoUrl: string;
  audioUrl: string | null;
  resolution: string | null;
  bandwidth: number;
  averageBandwidth: number | null;
  codecs: string | null;
  audioName: string | null;
}

export interface VariantSelectionItem {
  lineIndex: number;
  rawLine: string;
  url: string;
  title: string;
  selectedIndex: number;
  probe: M3U8ProbeResult;
}

export type TmdbMediaType = 'movie' | 'tv';

export interface TmdbSearchResult {
  id: number;
  mediaType: TmdbMediaType;
  title: string;
  originalTitle: string | null;
  year: string | null;
  seasonCount: number | null;
}

export interface TmdbEpisode {
  episodeNumber: number;
  name: string | null;
  airDate: string | null;
}

export interface TmdbSeasonDetails {
  seriesId: number;
  seasonNumber: number;
  episodes: TmdbEpisode[];
}

export interface M3U8NamingRow {
  lineIndex: number;
  url: string;
  originalTitle: string;
  generatedTitle: string;
  manualTitle: string;
  episodeNumber: number | null;
  episodeName: string | null;
}
