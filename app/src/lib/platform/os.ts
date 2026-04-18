export type Platform = "linux" | "macos" | "windows" | "android" | "ios";

export function platform(): Platform {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes("mac")) return "macos";
  if (platform.includes("win")) return "windows";
  if (platform.includes("linux")) return "linux";
  return "linux";
}

export function arch(): string {
  return navigator.userAgent.includes("arm64") ? "aarch64" : "x86_64";
}

export function version(): string {
  return navigator.userAgent;
}
