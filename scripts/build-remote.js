import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const buildArgs = [
  "build",
  "--features",
  "remote-only",
  "--",
  "--no-default-features",
];

// Local source builds normally do not have the release signing secret. Keep
// the installer build usable in that case; CI signs it when the secret exists.
if (!process.env.TAURI_SIGNING_PRIVATE_KEY?.trim()) {
  buildArgs.splice(1, 0, "--no-sign");
}

const result = spawnSync(
  process.execPath,
  [fileURLToPath(new URL("../node_modules/@tauri-apps/cli/tauri.js", import.meta.url)), ...buildArgs],
  {
    stdio: "inherit",
    env: {
      ...process.env,
      VITE_NEXQ_REMOTE_ONLY: "true",
    },
  },
);

if (result.error) {
  console.error(`Failed to start Tauri build: ${result.error.message}`);
  process.exitCode = 1;
} else {
  process.exitCode = result.status ?? 1;
}
