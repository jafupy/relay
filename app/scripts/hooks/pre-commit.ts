#!/usr/bin/env bun
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { $ } from "bun";

const APP_DIR = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
const REPO_ROOT = dirname(APP_DIR);
const BACKEND_DIR = join(REPO_ROOT, "backend");
const BIOME_EXTENSIONS = new Set([
  ".css",
  ".cjs",
  ".js",
  ".json",
  ".jsonc",
  ".jsx",
  ".mjs",
  ".ts",
  ".tsx",
]);

function extensionOf(file: string): string {
  const dot = file.lastIndexOf(".");
  return dot === -1 ? "" : file.slice(dot);
}

async function getStagedFiles(): Promise<string[]> {
  const output = await $`git -C ${REPO_ROOT} diff --cached --name-only --diff-filter=ACMR`.text();
  return output
    .split("\n")
    .map((file) => file.trim())
    .filter(Boolean);
}

async function main() {
  const stagedFiles = await getStagedFiles();

  if (stagedFiles.length === 0) {
    console.log("No staged files to validate.");
    return;
  }

  const rustFiles = stagedFiles.filter(
    (file) =>
      file.startsWith("backend/") &&
      (file.endsWith(".rs") || file.endsWith("Cargo.toml") || file.endsWith("Cargo.lock")),
  );
  const frontendFiles = stagedFiles
    .filter(
      (file) =>
        file.startsWith("app/") &&
        !file.startsWith("app/public/tree-sitter/parsers/") &&
        BIOME_EXTENSIONS.has(extensionOf(file)),
    )
    .map((file) => relative(APP_DIR, join(REPO_ROOT, file)));

  const checkedFiles = new Set([...rustFiles, ...frontendFiles.map((file) => `app/${file}`)]);
  const skippedFiles = stagedFiles.filter(
    (file) =>
      !checkedFiles.has(file) &&
      !file.startsWith("app/public/tree-sitter/parsers/") &&
      !file.startsWith("backend/target/"),
  );

  if (frontendFiles.length > 0) {
    console.log(`Running biome check --write on ${frontendFiles.length} staged file(s)...`);
    await $`bunx biome check --write ${frontendFiles}`.cwd(APP_DIR);
  }

  if (rustFiles.length > 0) {
    console.log("Running cargo fmt --all for staged Rust changes...");
    await $`cargo fmt --all`.cwd(BACKEND_DIR);
  }

  if (skippedFiles.length > 0) {
    console.log(`Skipping formatter for ${skippedFiles.length} staged file(s).`);
  }

  if (frontendFiles.length > 0 || rustFiles.length > 0 || skippedFiles.length > 0) {
    await $`git -C ${REPO_ROOT} update-index --again`;
  }

  await $`git -C ${REPO_ROOT} diff --cached --check`;
}

await main();
