import type { TaskCategory, TaskStatus } from '../types/app';

export const TASK_STATUS_LABELS: Record<TaskStatus, string> = {
  pending: '排队中',
  parsing: '解析中',
  downloading: '下载中',
  merging: '合并中',
  completed: '已完成',
  failed: '失败',
  paused: '已暂停',
  skipped: '已跳过',
  active: '进行中',
};

export const TASK_STATUS_BADGE_CLASS: Record<TaskStatus, string> = {
  pending: 'badge-ghost',
  parsing: 'badge-ghost',
  downloading: 'badge-info',
  merging: 'badge-warning',
  completed: 'badge-success',
  failed: 'badge-error',
  paused: 'badge-ghost',
  skipped: 'badge-neutral',
  active: 'badge-info',
};

export const TASK_CATEGORY_LABELS: Record<TaskCategory, string> = {
  movie: '电影',
  series: '剧集/综艺/动漫',
  other: '其它',
};

export const RESUMABLE_TASK_STATUSES: TaskStatus[] = ['paused', 'failed'];
export const PAUSABLE_PARENT_TASK_STATUSES: TaskStatus[] = ['active', 'downloading', 'merging', 'pending'];
export const PAUSABLE_SUBTASK_STATUSES: TaskStatus[] = ['downloading', 'pending', 'parsing'];

export const isResumableStatus = (status: TaskStatus) => RESUMABLE_TASK_STATUSES.includes(status);
export const isPausableParentStatus = (status: TaskStatus) => PAUSABLE_PARENT_TASK_STATUSES.includes(status);
export const isPausableSubtaskStatus = (status: TaskStatus) => PAUSABLE_SUBTASK_STATUSES.includes(status);
