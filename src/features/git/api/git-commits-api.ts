import { invoke as relayInvoke } from "@/lib/platform/core";
import type { GitCommit } from "../types/git-types";
import { isNotGitRepositoryError, resolveRepositoryPath } from "./git-repo-api";

export const commitChanges = async (repoPath: string, message: string): Promise<boolean> => {
  try {
    await relayInvoke("git_commit", { repoPath, message });
    return true;
  } catch (error) {
    console.error("Failed to commit changes:", error);
    return false;
  }
};

export const getGitLog = async (repoPath: string, limit = 50, skip = 0): Promise<GitCommit[]> => {
  try {
    const resolvedRepoPath = await resolveRepositoryPath(repoPath);
    if (!resolvedRepoPath) {
      return [];
    }

    const commits = await relayInvoke<GitCommit[]>("git_log", {
      repoPath: resolvedRepoPath,
      limit,
      skip,
    });
    return commits;
  } catch (error) {
    if (!isNotGitRepositoryError(error)) {
      console.error("Failed to get git log:", error);
    }
    return [];
  }
};
