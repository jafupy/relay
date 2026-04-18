import { invoke as relayInvoke } from "@/lib/platform/core";
import type { GitTag } from "../types/git-types";

export const getTags = async (repoPath: string): Promise<GitTag[]> => {
  try {
    const tags = await relayInvoke<GitTag[]>("git_get_tags", { repoPath });
    return tags;
  } catch (error) {
    console.error("Failed to get tags:", error);
    return [];
  }
};

export const createTag = async (
  repoPath: string,
  name: string,
  message?: string,
  commit?: string,
): Promise<boolean> => {
  try {
    await relayInvoke("git_create_tag", { repoPath, name, message, commit });
    return true;
  } catch (error) {
    console.error("Failed to create tag:", error);
    return false;
  }
};

export const deleteTag = async (repoPath: string, name: string): Promise<boolean> => {
  try {
    await relayInvoke("git_delete_tag", { repoPath, name });
    return true;
  } catch (error) {
    console.error("Failed to delete tag:", error);
    return false;
  }
};
