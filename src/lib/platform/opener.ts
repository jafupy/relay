import { invoke } from "./core";

export async function openUrl(url: string): Promise<void> {
  window.open(url, "_blank", "noopener,noreferrer");
}

export async function revealItemInDir(path: string): Promise<void> {
  await invoke("reveal_item_in_dir", { path });
}
