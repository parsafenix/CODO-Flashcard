import { existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";

const args = process.argv.slice(2);

if (args.length === 0) {
  console.error("Usage: node scripts/tauri-runner.mjs <dev|build> [...args]");
  process.exit(1);
}

const homeDirectory = process.platform === "win32" ? process.env.USERPROFILE : process.env.HOME;
const cargoBin = homeDirectory ? path.join(homeDirectory, ".cargo", "bin") : null;
const env = { ...process.env };
const pathEntries = [path.dirname(process.execPath)];

if (cargoBin && existsSync(cargoBin)) {
  pathEntries.push(cargoBin);
}

env.PATH = `${pathEntries.join(path.delimiter)}${env.PATH ? `${path.delimiter}${env.PATH}` : ""}`;

const tauriEntrypoint = path.join(process.cwd(), "node_modules", "@tauri-apps", "cli", "tauri.js");

const child = spawn(process.execPath, [tauriEntrypoint, ...args], {
  env,
  stdio: "inherit",
});

child.on("error", (error) => {
  console.error(`Failed to start the Tauri CLI: ${error.message}`);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code ?? 1);
});
