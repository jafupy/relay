import { describe, expect, it } from "vitest";
import { getChatTitleFromSessionInfo } from "./acp-session-info";

describe("getChatTitleFromSessionInfo", () => {
  it("returns trimmed title updates", () => {
    expect(getChatTitleFromSessionInfo("New Chat", "  Refactor parser  ")).toBe("Refactor parser");
  });

  it("ignores empty or unchanged titles", () => {
    expect(getChatTitleFromSessionInfo("New Chat", "   ")).toBeNull();
    expect(getChatTitleFromSessionInfo("Refactor parser", "Refactor parser")).toBeNull();
    expect(getChatTitleFromSessionInfo("Refactor parser", null)).toBeNull();
  });
});
