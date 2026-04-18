export async function invoke<T = unknown>(command: string, payload?: unknown): Promise<T> {
  const response = await fetch(`/api/rpc/${encodeURIComponent(command)}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(payload ?? {}),
  });

  const data = await response.json().catch(() => null);
  if (!response.ok || !data?.ok) {
    throw new Error(data?.error?.message || `Relay command failed: ${command}`);
  }

  return data.value as T;
}

export function convertFileSrc(path: string): string {
  return `/assets/file?path=${encodeURIComponent(path)}`;
}
