import { describe, expect, it } from "vitest";
import { parseGitHubEntityLink } from "./github-link-utils";

describe("parseGitHubEntityLink", () => {
  it("parses pull request links with extra path segments and fragments", () => {
    expect(
      parseGitHubEntityLink("https://github.com/relay/relay/pull/568/files#diff-123"),
    ).toMatchObject({
      kind: "pullRequest",
      owner: "relay",
      repo: "relay",
      number: 568,
    });
  });

  it("parses issue links with trailing slashes", () => {
    expect(parseGitHubEntityLink("https://github.com/relay/relay/issues/570/")).toMatchObject({
      kind: "issue",
      owner: "relay",
      repo: "relay",
      number: 570,
    });
  });

  it("parses action run links", () => {
    expect(
      parseGitHubEntityLink("https://github.com/relay/relay/actions/runs/23614391340"),
    ).toMatchObject({
      kind: "actionRun",
      owner: "relay",
      repo: "relay",
      runId: 23614391340,
    });
  });

  it("accepts www.github.com links", () => {
    expect(parseGitHubEntityLink("https://www.github.com/relay/relay/pull/568")).toMatchObject({
      kind: "pullRequest",
      owner: "relay",
      repo: "relay",
      number: 568,
    });
  });

  it("rejects non-GitHub hosts and malformed entity ids", () => {
    expect(parseGitHubEntityLink("https://example.com/relay/relay/pull/568")).toBeNull();
    expect(parseGitHubEntityLink("https://github.com/relay/relay/pull/not-a-number")).toBeNull();
  });
});
