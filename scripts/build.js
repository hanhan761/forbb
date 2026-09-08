import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const buildArgs = ["build"];

// Local source builds normally do not have the release signing secret. Keep
// the single shipped installer build usable in that case.
if (!process.env.TAURI_SIGNING_PRIVATE_KEY?.trim()) {
  buildArgs.push("--no-sign");
}

const result = spawnSync(
  process.execPath,
  [fileURLToPath(new URL("../node_modules/@tauri-apps/cli/tauri.js", import.meta.url)), ...buildArgs],
  {
    stdio: "inherit",
    env: process.env,
  },
);

if (result.error) {
  console.error(`Failed to start Tauri build: ${result.error.message}`);
  process.exitCode = 1;
} else {
  process.exitCode = result.status ?? 1;
}
