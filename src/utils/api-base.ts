const DEFAULT_API_BASE = "";

function isLocalApiBase(value: string): boolean {
  return value.includes("localhost") || value.includes("127.0.0.1");
}

export function getApiBase(): string {
  const configuredApiBase = import.meta.env.VITE_API_URL?.trim();

  if (!configuredApiBase) {
    return DEFAULT_API_BASE;
  }

  return configuredApiBase;
}

export const __test__ = {
  isLocalApiBase,
};
