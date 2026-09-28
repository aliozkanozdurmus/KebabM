import { useEffect, useRef, useState } from "react";
import { listAudioDevices, startAudioTest, stopAudioTest, testLLMConnection, testSTTConnection } from "../lib/ipc";
import { knowledge } from "../lib/knowledge";
import { useConfigStore } from "../stores/configStore";
import { useAudioLevel } from "../hooks/useAudioLevel";

type Check = "mic" | "system" | "model" | "stt" | "project";
const labels: Record<Check, string> = { mic: "Microphone", system: "Other participants / system audio", model: "Selected AI model", stt: "Transcription provider", project: "Project sources" };
export function ReadinessChecks({ projectId }: { projectId?: string }) {
  const [results, setResults] = useState<Partial<Record<Check, string>>>({});
  const [busy, setBusy] = useState<Check>();
  const mounted = useRef(true);
  const audioActive = useRef(false);
  const { micLevel, systemLevel } = useAudioLevel();
  const config = useConfigStore();
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; if (audioActive.current) void stopAudioTest(); }; }, []);
  useEffect(() => setResults({}), [projectId, config.llmProvider, config.llmModel, config.sttProvider, config.sttLanguage, config.micDeviceId, config.systemDeviceId]);
  const test = async (check: Check) => {
    setBusy(check); setResults(old => ({ ...old, [check]: "Testing…" }));
    try {
      let result: string;
      if (check === "mic" || check === "system") {
        const devices = await listAudioDevices();
        const isInput = check === "mic";
        const selected = isInput ? config.micDeviceId : config.systemDeviceId;
        const list = isInput ? devices.inputs : devices.outputs;
        const device = selected || list.find(d => d.is_default)?.id || "default";
        audioActive.current = true;
        await startAudioTest(device, isInput);
        if (!mounted.current) { await stopAudioTest(); audioActive.current = false; return; }
        await new Promise(resolve => setTimeout(resolve, 4000));
        result = await stopAudioTest() ? "Signal detected" : "No signal detected. Speak or play meeting audio and test again.";
        audioActive.current = false;
      } else if (check === "model") {
        result = await testLLMConnection("{}", config.llmModel) ? `Responded · ${config.llmModel}` : "Model did not respond";
      } else if (check === "stt") {
        result = await testSTTConnection(config.sttProvider) ? "Connection checked · live transcription still needs a speech test" : "Provider unavailable";
      } else if (projectId) {
        const c = await knowledge.coverage(projectId);
        const freshness = c.freshness === "current" ? "Matches local sources" : c.freshness === "stale" ? "Sources changed · update index before joining" : "Local sources unavailable · using saved snapshot";
        result = `${freshness} · ${c.indexedFiles} files · ${new Date(c.scannedAt).toLocaleString()}`;
      } else { result = "General meeting · no project selected"; }
      if (mounted.current) setResults(old => ({ ...old, [check]: result }));
    } catch (error) { if (audioActive.current) { await stopAudioTest().catch(() => {}); audioActive.current = false; } if (mounted.current) setResults(old => ({ ...old, [check]: String(error) })); }
    finally { if (mounted.current) setBusy(undefined); }
  };
  return <section className="border-t border-border px-3 py-3" aria-label="Readiness checks"><h3 className="text-sm font-medium">Check before joining</h3><p className="mt-1 mb-3 text-xs text-muted-foreground">Speak for the microphone test; play Zoom or other audio for the system test. AI testing makes one small request to the selected model.</p><div className="space-y-3">{(Object.keys(labels) as Check[]).map(check => <div key={check} className="flex gap-2 items-start"><div className="min-w-0 flex-1"><p className="text-xs font-medium">{labels[check]}</p><p className="mt-1 text-xs text-muted-foreground break-words" role="status">{results[check] || "Not tested"}</p>{busy === check && (check === "mic" || check === "system") && <meter aria-label={`${labels[check]} level`} min={0} max={1} value={check === "mic" ? micLevel : systemLevel} className="w-full" />}</div><button className="border border-border px-2 py-1 text-xs disabled:opacity-50 hover:bg-accent" disabled={!!busy} onClick={() => test(check)}>Test</button></div>)}</div></section>;
}
