import { invoke as relayInvoke } from "@/lib/platform/core";
import type { GitRemote } from "../types/git-types";

export interface GitRemoteActionResult {
  success: boolean;
  error?: string;
}

export const getRemotes = async (repoPath: string): Promise<GitRemote[]> => {
  try {
    const remotes = await relayInvoke<GitRemote[]>("git_get_remotes", {
      repoPath,
    });
    return remotes;
  } catch (error) {
    console.error("Failed to get remotes:", error);
    return [];
  }
};

export const addRemote = async (repoPath: string, name: string, url: string): Promise<boolean> => {
  try {
    await relayInvoke("git_add_remote", { repoPath, name, url });
    return true;
  } catch (error) {
    console.error("Failed to add remote:", error);
    return false;
  }
};

export const removeRemote = async (repoPath: string, name: string): Promise<boolean> => {
  try {
    await relayInvoke("git_remove_remote", { repoPath, name });
    return true;
  } catch (error) {
    console.error("Failed to remove remote:", error);
    return false;
  }
};

export const pushChanges = async (
  repoPath: string,
  branch?: string,
  remote: string = "origin",
): Promise<GitRemoteActionResult> => {
  try {
    await relayInvoke("git_push", { repoPath, branch, remote });
    return { success: true };
  } catch (error) {
    console.error("Failed to push changes:", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
};

export const pullChanges = async (
  repoPath: string,
  branch?: string,
  remote: string = "origin",
): Promise<GitRemoteActionResult> => {
  try {
    await relayInvoke("git_pull", { repoPath, branch, remote });
    return { success: true };
  } catch (error) {
    console.error("Failed to pull changes:", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
};

export const fetchChanges = async (
  repoPath: string,
  remote?: string,
): Promise<GitRemoteActionResult> => {
  try {
    await relayInvoke("git_fetch", { repoPath, remote });
    return { success: true };
  } catch (error) {
    console.error("Failed to fetch changes:", error);
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
};
