import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { listAudioDevices, listLocalSTTEngines, startAudioTest, stopAudioTest, testLLMConnection, testSTTConnection } from "../lib/ipc";
import { knowledge } from "../lib/knowledge";
import { useConfigStore } from "../stores/configStore";
import { useAudioLevel } from "../hooks/useAudioLevel";

type Check = "mic" | "system" | "model" | "youStt" | "themStt" | "project";
const labels: Record<Check, string> = { mic: "Microphone", system: "Other participants / system audio", model: "Selected AI model", youStt: "Your transcription", themStt: "Other participants’ transcription", project: "Project sources" };
export function ReadinessChecks({ projectId }: { projectId?: string }) {
  const [results, setResults] = useState<Partial<Record<Check, string>>>({});
  const [busy, setBusy] = useState<Check>();
  const mounted = useRef(true);
  const audioActive = useRef(false);
  const settingsVersion = useRef(0);
  const running = useRef(false);
  const { micLevel, systemLevel } = useAudioLevel();
  const config = useConfigStore();
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; if (audioActive.current) void stopAudioTest(); }; }, []);
  useLayoutEffect(() => {
    settingsVersion.current += 1;
    setResults({});
  }, [projectId, config.llmProvider, config.llmModel, config.sttProvider, config.sttLanguage, config.micDeviceId, config.systemDeviceId, config.meetingAudioConfig, config.activeWhisperModel, config.activeModelPerEngine]);
  const test = async (check: Check) => {
    if (running.current) return;
    running.current = true;
    const version = settingsVersion.current;
    const isCurrent = () => mounted.current && version === settingsVersion.current;
    setBusy(check); setResults(old => ({ ...old, [check]: "Testing…" }));
    try {
      let result: string;
      if (check === "mic" || check === "system") {
        const devices = await listAudioDevices();
        const party = check === "mic" ? config.meetingAudioConfig?.you : config.meetingAudioConfig?.them;
        const isInput = party?.is_input_device ?? (check === "mic");
        const selected = party?.device_id || (isInput ? config.micDeviceId : config.systemDeviceId);
        const list = isInput ? devices.inputs : devices.outputs;
        const device = selected || list.find(d => d.is_default)?.id || "default";
        audioActive.current = true;
        await startAudioTest(device, isInput);
        if (!isCurrent()) { await stopAudioTest(); audioActive.current = false; return; }
        await new Promise(resolve => setTimeout(resolve, 4000));
        result = await stopAudioTest() ? "Signal detected" : "No signal detected. Speak or play meeting audio and test again.";
        audioActive.current = false;
      } else if (check === "model") {
        result = await testLLMConnection("{}", config.llmModel) ? `Responded · ${config.llmModel}` : "Model did not respond";
      } else if (check === "youStt" || check === "themStt") {
        const party = check === "youStt" ? config.meetingAudioConfig?.you : config.meetingAudioConfig?.them;
        const provider = party?.stt_provider ?? config.sttProvider;
        if (provider === "web_speech") {
          if (party && !party.is_input_device) throw new Error("Web Speech only receives microphone input. Choose another provider for system audio in Audio settings.");
          result = (window.SpeechRecognition || window.webkitSpeechRecognition)
            ? "Web Speech supported · microphone permission and live speech still need a test"
            : "Web Speech is unavailable in this window. Choose a supported transcription provider in Audio settings.";
        } else if (["whisper_cpp", "sherpa_onnx", "ort_streaming", "parakeet_tdt", "moonshine"].includes(provider)) {
          const engine = (await listLocalSTTEngines()).find(item => item.engine === provider);
          const model = engine?.models.find(item => item.is_downloaded && (!party?.local_model_id || item.id === party.local_model_id));
          result = model ? `${model.name} downloaded · live recognition still needs a speech test`
            : "Selected local model is not downloaded. Download it in Audio settings before using transcription.";
        } else {
          result = await testSTTConnection(provider) ? `${provider.replaceAll("_", " ")} · connection checked; live speech still needs a test` : "Provider unavailable";
        }
      } else if (projectId) {
        const c = await knowledge.coverage(projectId);
        const freshness = c.freshness === "current" ? "Matches local sources" : c.freshness === "stale" ? "Sources changed · update index before joining" : (c.freshnessError || "Local sources unavailable · using saved snapshot");
        result = `${freshness} · ${c.indexedFiles} files · ${new Date(c.scannedAt).toLocaleString()}`;
      } else { result = "General meeting · no project selected"; }
      if (isCurrent()) setResults(old => ({ ...old, [check]: result }));
    } catch (error) { if (audioActive.current) { await stopAudioTest().catch(() => {}); audioActive.current = false; } if (isCurrent()) setResults(old => ({ ...old, [check]: String(error) })); }
    finally { running.current = false; if (mounted.current) setBusy(undefined); }
  };
  return <section className="border-t border-border px-3 py-3" aria-label="Readiness checks"><h3 className="text-sm font-medium">Check before joining</h3><p className="mt-1 mb-3 text-xs text-muted-foreground">Speak for the microphone test; play Zoom or other audio for the system test. AI testing makes one small request to the selected model.</p><div className="space-y-3">{(Object.keys(labels) as Check[]).map(check => <div key={check} className="flex gap-2 items-start"><div className="min-w-0 flex-1"><p className="text-xs font-medium">{labels[check]}</p><p className="mt-1 text-xs text-muted-foreground break-words" role="status">{results[check] || "Not tested"}</p>{busy === check && (check === "mic" || check === "system") && <meter aria-label={`${labels[check]} level`} min={0} max={1} value={check === "mic" ? micLevel : systemLevel} className="w-full" />}</div><button className="border border-border px-2 py-1 text-xs disabled:opacity-50 hover:bg-accent" disabled={!!busy} aria-label={`Test ${labels[check]}`} onClick={() => test(check)}>Test</button></div>)}</div></section>;
}
