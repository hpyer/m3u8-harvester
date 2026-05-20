import fs from 'node:fs';
import path from 'node:path';

const rootDir = path.resolve(import.meta.dirname, '..');
const cargoTomlPath = path.join(rootDir, 'Cargo.toml');

function readWorkspaceVersion() {
  const content = fs.readFileSync(cargoTomlPath, 'utf8');
  const match = content.match(/^\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m);

  if (!match) {
    throw new Error('Failed to find [workspace.package].version in Cargo.toml');
  }

  return match[1];
}

function setWorkspaceVersion(nextVersion) {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(nextVersion)) {
    throw new Error(`Invalid semver version: ${nextVersion}`);
  }

  const content = fs.readFileSync(cargoTomlPath, 'utf8');
  const currentVersion = readWorkspaceVersion();
  if (currentVersion === nextVersion) {
    return nextVersion;
  }

  const updated = content.replace(
    /^(\[workspace\.package\][\s\S]*?^version = )"([^"]+)"/m,
    `$1"${nextVersion}"`,
  );

  if (updated === content) {
    throw new Error('Failed to update [workspace.package].version in Cargo.toml');
  }

  fs.writeFileSync(cargoTomlPath, updated);
  return nextVersion;
}

const [command, value] = process.argv.slice(2);

switch (command) {
  case 'get':
    console.log(readWorkspaceVersion());
    break;
  case 'set':
    if (!value) {
      throw new Error('Usage: pnpm version:set <x.y.z>');
    }
    console.log(setWorkspaceVersion(value));
    break;
  default:
    throw new Error('Usage: pnpm version:get | pnpm version:set <x.y.z>');
}
