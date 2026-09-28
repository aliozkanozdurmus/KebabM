import { describe, expect, it } from "vitest";
import { RequestGate } from "./requestGate";
describe("request isolation", () => {
  it("ignores late tokens and starts after a cancelled network stream", () => {
    const g = new RequestGate(); g.begin("old"); g.start("old"); g.cancel(); g.begin("new");
    expect(g.start("old")).toBe(false); expect(g.token("old")).toBe(false); expect(g.end("old")).toBe(false);
    expect(g.start("new")).toBe(true); expect(g.token("new")).toBe(true);
  });
  it("allows every subscriber to observe completion without ending a newer request", () => {
    const g = new RequestGate(); g.start("one"); expect(g.end("one")).toBe(true); expect(g.end("one")).toBe(true);
    g.start("two"); expect(g.end("one")).toBe(false); expect(g.token("two")).toBe(true);
  });
  it("rejects background and unscoped events during a live request", () => {
    const g = new RequestGate(); g.begin("live"); expect(g.start("scan")).toBe(false); expect(g.token()).toBe(false); expect(g.end()).toBe(false);
  });
});
