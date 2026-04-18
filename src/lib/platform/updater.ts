export interface DownloadEvent {
  event: "Started" | "Progress" | "Finished";
  data: {
    contentLength?: number;
    chunkLength: number;
  };
}

export interface Update {
  version: string;
  currentVersion: string;
  available: boolean;
  body?: string;
  date?: string;
  downloadAndInstall: (onEvent?: (event: DownloadEvent) => void) => Promise<void>;
}

export async function check(): Promise<Update | null> {
  return null;
}
