import { beforeEach, expect, it } from "vitest";
import { useStreamStore } from "./streamStore";
beforeEach(() => { useStreamStore.getState().clearCurrent(); useStreamStore.setState({ responseHistory: [], pinnedResponses: [] }); });
it("retains the answer being read when a new question starts or fails", () => {
  const s = useStreamStore.getState(); s.startStream("Assist", "model", "provider"); s.appendToken("Mailbox feeds the worker."); s.endStream(1250, 80);
  s.startStream("Assist", "model", "provider"); s.appendToken("partial"); s.setError("Provider quota reached");
  expect(useStreamStore.getState().responseHistory.map(x => x.content)).toEqual(["Mailbox feeds the worker."]);
  expect(useStreamStore.getState().isStreaming).toBe(false);
  s.startStream("Assist", "model", "provider"); s.appendToken("Retry succeeded."); s.endStream(1000);
  expect(useStreamStore.getState().responseHistory).toHaveLength(2);
});
it("never saves empty or duplicate terminal responses", () => {
  const s = useStreamStore.getState(); s.startStream("Assist", "model", "provider"); s.endStream(0);
  expect(useStreamStore.getState().responseHistory).toHaveLength(0);
  s.startStream("Assist", "model", "provider"); s.appendToken("Useful answer."); s.endStream(10); s.endStream(10);
  expect(useStreamStore.getState().responseHistory).toHaveLength(1);
});
it("merges the durable session snapshot with its terminal stream event", () => {
  const s = useStreamStore.getState();
  s.startStream("Assist", "model", "provider");
  useStreamStore.setState({ requestId: "same-request", sessionId: "meeting" });
  s.appendToken("Persisted answer.");
  useStreamStore.setState({ responseHistory: [{ id: "same-request", content: "Persisted answer.", mode: "Assist", timestamp: Date.now(), pinned: false, model: "model", provider: "provider", latency_ms: 100 }] });
  s.endStream(100, 20);
  expect(useStreamStore.getState().responseHistory).toHaveLength(1);
  expect(useStreamStore.getState().responseHistory[0].totalTokens).toBe(20);
});
