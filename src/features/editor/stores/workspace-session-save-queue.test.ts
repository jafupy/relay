import { describe, expect, it } from "vitest";
import { createWorkspaceSessionSaveQueue } from "./workspace-session-save-queue";

describe("createWorkspaceSessionSaveQueue", () => {
  const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  type Payload = { activeBufferId: string };

  it("keeps saves isolated per workspace", async () => {
    const calls: Array<[string, Payload]> = [];
    const queue = createWorkspaceSessionSaveQueue<Payload>((...args) => calls.push(args), 1);

    queue.schedule("/workspace-a", { activeBufferId: "a1" });
    queue.schedule("/workspace-b", { activeBufferId: "b1" });

    await wait(5);

    expect(calls).toEqual([
      ["/workspace-a", { activeBufferId: "a1" }],
      ["/workspace-b", { activeBufferId: "b1" }],
    ]);
  });

  it("coalesces repeated saves for the same workspace", async () => {
    const calls: Array<[string, Payload]> = [];
    const queue = createWorkspaceSessionSaveQueue<Payload>((...args) => calls.push(args), 5);

    queue.schedule("/workspace-a", { activeBufferId: "a1" });
    await wait(1);
    queue.schedule("/workspace-a", { activeBufferId: "a2" });
    await wait(10);

    expect(calls).toEqual([["/workspace-a", { activeBufferId: "a2" }]]);
  });

  it("can clear a queued save before it flushes", async () => {
    const calls: Array<[string, Payload]> = [];
    const queue = createWorkspaceSessionSaveQueue<Payload>((...args) => calls.push(args), 1);

    queue.schedule("/workspace-a", { activeBufferId: "a1" });
    queue.clear("/workspace-a");
    await wait(5);

    expect(calls).toEqual([]);
  });
});
