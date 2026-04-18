import { invoke } from "./core";

interface DialogFilter {
  name: string;
  extensions: string[];
}

interface OpenDialogOptions {
  directory?: boolean;
  multiple?: boolean;
  filters?: DialogFilter[];
  title?: string;
  defaultPath?: string;
}

interface SaveDialogOptions {
  filters?: DialogFilter[];
  title?: string;
  defaultPath?: string;
}

export async function open(options?: OpenDialogOptions): Promise<string | string[] | null> {
  return invoke<string | string[] | null>("dialog_open", options ?? {});
}

export async function save(options?: SaveDialogOptions): Promise<string | null> {
  return invoke<string | null>("dialog_save", options ?? {});
}

export async function confirm(message: string, _options?: unknown): Promise<boolean> {
  return window.confirm(message);
}

export async function ask(message: string, _options?: unknown): Promise<boolean> {
  return window.confirm(message);
}
