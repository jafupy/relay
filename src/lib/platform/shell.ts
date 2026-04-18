export async function open(url: string, _openWith?: string): Promise<void> {
  window.open(url, "_blank", "noopener,noreferrer");
}
