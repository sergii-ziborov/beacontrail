'use strict';

/**
 * prepublishOnly: bundle the release engine into the package.
 *
 * Publishing without a fresh binary must fail loudly, not ship a stale or
 * missing engine.
 */

const { copyFileSync, statSync } = require('node:fs');
const { join } = require('node:path');
const { spawnSync } = require('node:child_process');
const packageJson = require('./package.json');

const source = join(__dirname, '..', 'target', 'release', 'radiochron.exe');
const target = join(__dirname, 'radiochron.exe');

// The dual license rides along with the tarball.
for (const name of ['LICENSE-MIT', 'LICENSE-APACHE']) {
  copyFileSync(join(__dirname, '..', name), join(__dirname, name));
}

let size;
try {
  size = statSync(source).size;
} catch {
  console.error('prepare: no release binary. Run `cargo build --release` first.');
  process.exit(1);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: join(__dirname, '..'),
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || '').trim();
    throw new Error(`${command} ${args.join(' ')} failed${detail ? `: ${detail}` : ''}`);
  }
  return result.stdout.trim();
}

let buildInfo;
let head;
let dirty;
try {
  buildInfo = JSON.parse(run(source, ['--build-info']));
  head = run('git', ['rev-parse', 'HEAD']);
  dirty = run('git', ['status', '--porcelain', '--untracked-files=all']);
} catch (error) {
  console.error(`prepare: cannot verify binary provenance: ${error.message}`);
  process.exit(1);
}
if (dirty) {
  console.error('prepare: working tree is not clean. Commit the exact release sources before packaging.');
  process.exit(1);
}

if (buildInfo.version !== packageJson.version) {
  console.error(`prepare: binary version ${buildInfo.version} != npm version ${packageJson.version}`);
  process.exit(1);
}
if (buildInfo.git_sha !== head) {
  console.error(`prepare: binary was built from ${buildInfo.git_sha}, current HEAD is ${head}. Rebuild from this commit.`);
  process.exit(1);
}

copyFileSync(source, target);
console.log(`prepare: bundled radiochron.exe (${Math.round(size / 1024)} KB, ${head.slice(0, 12)})`);
