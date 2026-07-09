import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const desktopDir = resolve(rootDir, 'apps/desktop');
const sourceIcon = resolve(rootDir, 'apps/desktop/src-tauri/icons/icon.png');
const webIcon = resolve(rootDir, 'apps/web/public/icon.png');
const iconStamp = resolve(rootDir, 'apps/desktop/src-tauri/icon-source.sha256');

const desktopOutputs = [
  'src-tauri/icons/32x32.png',
  'src-tauri/icons/128x128.png',
  'src-tauri/icons/128x128@2x.png',
  'src-tauri/icons/icon.icns',
  'src-tauri/icons/icon.ico',
].map((path) => resolve(desktopDir, path));
const tauriBin = resolve(
  desktopDir,
  process.platform === 'win32' ? 'node_modules/.bin/tauri.cmd' : 'node_modules/.bin/tauri',
);

const sha256 = (path) => createHash('sha256').update(readFileSync(path)).digest('hex');

const sameFileContent = (left, right) => existsSync(right) && sha256(left) === sha256(right);

const syncWebIcon = () => {
  mkdirSync(dirname(webIcon), { recursive: true });

  if (sameFileContent(sourceIcon, webIcon)) {
    console.log('Web icon is unchanged.');
    return;
  }

  writeFileSync(webIcon, readFileSync(sourceIcon));
  console.log('Updated apps/web/public/icon.png.');
};

const readStamp = () => {
  if (!existsSync(iconStamp)) return null;
  return readFileSync(iconStamp, 'utf8').trim();
};

const writeStamp = () => {
  writeFileSync(iconStamp, `${sha256(sourceIcon)}\n`);
};

const generateDesktopIcons = () => {
  const currentHash = sha256(sourceIcon);
  const outputsExist = desktopOutputs.every((path) => existsSync(path));

  if (readStamp() === currentHash && outputsExist) {
    console.log('Desktop icons are unchanged.');
    return;
  }

  const result = spawnSync(
    existsSync(tauriBin) ? tauriBin : 'tauri',
    ['icon', 'src-tauri/icons/icon.png', '-o', 'src-tauri/icons'],
    {
      cwd: desktopDir,
      stdio: 'inherit',
    },
  );

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }

  writeStamp();
};

const command = process.argv[2];

if (command === 'web') {
  syncWebIcon();
} else if (command === 'desktop') {
  generateDesktopIcons();
} else {
  console.error('Usage: node scripts/icons.mjs <web|desktop>');
  process.exit(1);
}
