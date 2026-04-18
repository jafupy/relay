export async function appDataDir(): Promise<string> {
  return ".relay";
}

export async function basename(path: string): Promise<string> {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

export async function dirname(path: string): Promise<string> {
  const parts = path.split(/[\\/]/);
  parts.pop();
  return parts.join(path.includes("\\") ? "\\" : "/") || ".";
}

export async function extname(path: string): Promise<string> {
  const name = await basename(path);
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index) : "";
}

export async function join(...parts: string[]): Promise<string> {
  return parts.filter(Boolean).join("/");
}
