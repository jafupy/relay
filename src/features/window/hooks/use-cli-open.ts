import { useEffect } from "react";
import { listen } from "@/lib/platform/events";
import { handleWindowOpenRequest } from "../utils/window-open-request";

interface CliOpenPayload {
  kind: "path" | "web" | "terminal" | "remote";
  path?: string;
  is_directory?: boolean;
  line?: number | null;
  url?: string;
  command?: string | null;
  working_directory?: string | null;
  connection_id?: string;
  name?: string | null;
}

export function useCliOpen() {
  useEffect(() => {
    const unlisten = listen<CliOpenPayload>("cli_open_request", (event) => {
      const payload = event.payload;

      switch (payload.kind) {
        case "web":
          if (!payload.url) return;
          handleWindowOpenRequest({
            type: "web",
            url: payload.url,
          });
          break;
        case "terminal":
          handleWindowOpenRequest({
            type: "terminal",
            command: payload.command ?? undefined,
            workingDirectory: payload.working_directory ?? undefined,
          });
          break;
        case "remote":
          if (!payload.connection_id) return;
          handleWindowOpenRequest({
            type: "remote",
            remoteConnectionId: payload.connection_id,
            remoteConnectionName: payload.name ?? undefined,
          });
          break;
        case "path":
        default:
          if (!payload.path) return;
          handleWindowOpenRequest({
            type: "path",
            path: payload.path,
            isDirectory: payload.is_directory ?? false,
            line: payload.line ?? undefined,
          });
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
