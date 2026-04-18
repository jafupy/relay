export async function getVersion(): Promise<string> {
  const response = await fetch("/api/version");
  const data = await response.json();
  return data.version ?? "0.0.0";
}
