import { describe, expect, it } from "vitest";
import { __test__ as apiBaseTest } from "@/utils/api-base";

describe("auth-api local auth helpers", () => {
  it("detects local api base URLs", () => {
    expect(apiBaseTest.isLocalApiBase("http://localhost:3000")).toBe(true);
    expect(apiBaseTest.isLocalApiBase("http://127.0.0.1:3000")).toBe(true);
    expect(apiBaseTest.isLocalApiBase("https://relay.local")).toBe(false);
  });
});
