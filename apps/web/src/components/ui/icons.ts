export const ICON_NAMES = [
  'settings',
  'sun',
  'moon',
  'warning',
  'tasks',
  'files',
  'plus',
  'play',
  'pause',
  'trash',
  'refresh',
  'copy',
  'edit',
  'chevron-right',
  'chevron-up',
  'chevron-down',
] as const;

export type IconName = (typeof ICON_NAMES)[number];
