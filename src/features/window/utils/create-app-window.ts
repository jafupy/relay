import type { WindowOpenRequest } from "@/features/window/utils/window-open-request";
import { invoke } from "@/lib/platform/core";

interface CreateAppWindowPayload {
  request?: WindowOpenRequest | null;
}

export async function createAppWindow(request?: WindowOpenRequest | null) {
  return invoke<string>("create_app_window", {
    request: request ?? null,
  } satisfies CreateAppWindowPayload);
}
