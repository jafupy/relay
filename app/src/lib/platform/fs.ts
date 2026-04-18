import { invoke } from "./core";

export enum BaseDirectory {
  AppData = "AppData",
}

export async function readFile(path: string, _options?: unknown): Promise<Uint8Array<ArrayBuffer>> {
  const content = await invoke<string>("read_file", { path });
  return new TextEncoder().encode(content);
}

export async function writeTextFile(
  path: string,
  content: string,
  _options?: unknown,
): Promise<void> {
  await invoke("write_file", { path, content });
}

export async function writeFile(
  path: string,
  content: Uint8Array,
  _options?: unknown,
): Promise<void> {
  await invoke("write_file", { path, content: new TextDecoder().decode(content) });
}

export async function readDir(
  path: string,
): Promise<Array<{ name: string; isDirectory: boolean }>> {
  return invoke("read_directory", { path });
}

export async function mkdir(path: string, _options?: unknown): Promise<void> {
  await invoke("create_directory", { path });
}

export async function remove(path: string, _options?: unknown): Promise<void> {
  await invoke("delete_path", { path });
}

export async function copyFile(
  sourcePath: string,
  targetPath: string,
  _options?: unknown,
): Promise<void> {
  await invoke("copy_file", { sourcePath, targetPath });
}

export async function exists(path: string, _options?: unknown): Promise<boolean> {
  try {
    await invoke("get_symlink_info", { path });
    return true;
  } catch {
    return false;
  }
}
