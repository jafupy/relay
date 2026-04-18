export interface WindowOpenRequest {
  type?: "path" | "remote" | "web" | "terminal";
  path?: string;
  isDirectory?: boolean;
  line?: number;
  remoteConnectionId?: string;
  remoteConnectionName?: string;
  url?: string;
  command?: string;
  workingDirectory?: string;
}

export function parseWindowOpenUrl(url: URL): WindowOpenRequest | null {
  const target = url.searchParams.get("target");
  if (target !== "open" && url.host !== "open") return null;

  const type = url.searchParams.get("type");
  if (type === "remote") {
    const remoteConnectionId = url.searchParams.get("connectionId");
    if (!remoteConnectionId) return null;

    return {
      type: "remote",
      remoteConnectionId,
      remoteConnectionName: url.searchParams.get("name") ?? undefined,
    };
  }

  if (type === "web") {
    const webUrl = url.searchParams.get("url");
    if (!webUrl) return null;

    return {
      type: "web",
      url: webUrl,
    };
  }

  if (type === "terminal") {
    return {
      type: "terminal",
      command: url.searchParams.get("command") ?? undefined,
      workingDirectory: url.searchParams.get("cwd") ?? undefined,
    };
  }

  const path = url.searchParams.get("path");
  if (!path) return null;

  const lineParam = url.searchParams.get("line");
  const line = lineParam ? Number.parseInt(lineParam, 10) : undefined;

  return {
    type: "path",
    path,
    isDirectory: type === "directory",
    line: line && line > 0 ? line : undefined,
  };
}

export async function handleWindowOpenRequest(request: WindowOpenRequest) {
  const { useBufferStore } = await import("@/features/editor/stores/buffer-store");
  const { useFileSystemStore } = await import("@/features/file-system/controllers/store");
  const { handleFileSelect, handleOpenFolderByPath, handleOpenRemoteProject } =
    useFileSystemStore.getState();

  if (request.type === "web" && request.url) {
    useBufferStore.getState().actions.openWebViewerBuffer(request.url);
    return;
  }

  if (request.type === "terminal") {
    useBufferStore.getState().actions.openTerminalBuffer({
      command: request.command,
      workingDirectory: request.workingDirectory,
    });
    return;
  }

  if (request.type === "remote" && request.remoteConnectionId) {
    await handleOpenRemoteProject(
      request.remoteConnectionId,
      request.remoteConnectionName ?? "Remote",
    );
    return;
  }

  if (!request.path) return;

  if (request.isDirectory) {
    await handleOpenFolderByPath(request.path);
  } else {
    await handleFileSelect(request.path, false, request.line);
  }
}

export const __test__ = { parseWindowOpenUrl };
