import { expect, it, vi } from "vitest";

const saved = vi.hoisted(() => new Map<string, unknown>());
vi.mock("@tauri-apps/plugin-store", () => ({
  load: async () => ({
    get: async (key: string) => structuredClone(saved.get(key)),
    set: async (key: string, value: unknown) => saved.set(key, structuredClone(value)),
    save: async () => {},
    onKeyChange: async () => () => {},
  }),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: async () => null }));

import { useConfigStore } from "./configStore";

it("preserves both local Whisper selections and independent languages across startup", async () => {
  const audio = {
    you: { role: "You", device_id: "default", is_input_device: true, stt_provider: "whisper_cpp", local_model_id: "small" },
    them: { role: "Them", device_id: "speakers", is_input_device: false, stt_provider: "whisper_cpp", local_model_id: "small" },
    recording_enabled: false,
    preset_name: null,
  };
  saved.set("meetingAudioConfig", audio);
  saved.set("sttLanguage", "tr-TR");
  saved.set("aiReplyLanguage", "en");
  useConfigStore.setState({ _loaded: false });
  await useConfigStore.getState().loadConfig();
  expect(useConfigStore.getState().meetingAudioConfig).toEqual(audio);
  expect(saved.get("meetingAudioConfig")).toEqual(audio);
  expect(useConfigStore.getState().sttLanguage).toBe("tr-TR");
  expect(useConfigStore.getState().aiReplyLanguage).toBe("en");
});
