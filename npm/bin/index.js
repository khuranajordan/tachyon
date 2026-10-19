#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

function getBinaryPath() {
  const platform = os.platform();
  const binDir = __dirname;

  if (platform === 'darwin') {
    const universalBin = path.join(binDir, 'tachyon-darwin-universal');
    if (fs.existsSync(universalBin)) {
      return universalBin;
    }
  }

  // Check if system has tachyon or cargo-installed tachyon
  const fallbackPaths = [
    path.join(binDir, platform === 'win32' ? 'tachyon.exe' : 'tachyon'),
    path.join(os.homedir(), '.cargo', 'bin', platform === 'win32' ? 'tachyon.exe' : 'tachyon')
  ];

  for (const p of fallbackPaths) {
    if (fs.existsSync(p)) {
      return p;
    }
  }

  return 'tachyon';
}

const binary = getBinaryPath();
const args = process.argv.slice(2);

try {
  const child = spawn(binary, args, {
    stdio: 'inherit',
    env: process.env
  });

  child.on('error', (err) => {
    if (err.code === 'ENOENT') {
      console.error('\x1b[1;31merror:\x1b[0m Tachyon native binary not found for your platform.');
      console.error('Please visit https://github.com/khuranajordan/tachyon to install or build from source.');
    } else {
      console.error('\x1b[1;31merror:\x1b[0m Failed to execute tachyon:', err.message);
    }
    process.exit(1);
  });

  child.on('exit', (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code ?? 0);
    }
  });

  // Forward termination signals
  ['SIGINT', 'SIGTERM', 'SIGHUP'].forEach((sig) => {
    process.on(sig, () => {
      if (!child.killed) {
        child.kill(sig);
      }
    });
  });
} catch (e) {
  console.error('\x1b[1;31merror:\x1b[0m', e.message);
  process.exit(1);
}
