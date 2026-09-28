import { useEffect, useState } from "react";
import { listProjects, setActiveProject, type ProjectRecord } from "../lib/ipc";
import { ReadinessChecks } from "./ReadinessChecks";
import { Play, X } from "lucide-react";

const GENERAL = "general";

interface MeetingSetupModalProps {
  open: boolean;
  meetingTitle?: string;
  onStart: (choice: { projectId: string } | { context: string }) => void;
  onCancel: () => void;
}

export function MeetingSetupModal({ open, meetingTitle, onStart, onCancel }: MeetingSetupModalProps) {
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [selectedId, setSelectedId] = useState<string>(GENERAL);
  const [context, setContext] = useState("");
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!open) return;
    listProjects()
      .then((rows) => {
        setProjects(rows);
        const active = rows.find((project) => project.is_active);
        setSelectedId(active?.id ?? GENERAL);
      })
      .catch(() => setProjects([]));
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handle = (event: KeyboardEvent) => {
      if (event.key === "Escape") onCancel();
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [open, onCancel]);

  if (!open) return null;

  const start = async () => {
    if (starting) return;
    setStarting(true);
    try {
      if (selectedId === GENERAL) {
        await setActiveProject(null);
        onStart({ context });
      } else {
        await setActiveProject(selectedId);
        onStart({ projectId: selectedId });
      }
    } catch (e) { setError(String(e)); } finally {
      setStarting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/65"
      role="dialog"
      aria-modal="true"
      aria-label="Choose a project"
      onClick={(event) => {
        if (event.target === event.currentTarget) onCancel();
      }}
    >
      <div className="w-[480px] max-w-[calc(100vw-32px)] max-h-[92vh] overflow-auto border border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <div>
            <h2 className="text-sm font-semibold text-foreground">{meetingTitle || "Start meeting"}</h2>
            <p className="text-xs text-muted-foreground">Pick a project, or General and paste the context for this meeting.</p>
          </div>
          <button onClick={onCancel} className="p-1 text-muted-foreground hover:text-foreground cursor-pointer" aria-label="Cancel">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="max-h-60 space-y-1 overflow-y-auto px-3 py-3">
          <button
            onClick={() => setSelectedId(GENERAL)}
            className={`block w-full border px-3 py-2 text-left cursor-pointer ${selectedId === GENERAL ? "border-primary bg-primary text-primary-foreground" : "border-border text-foreground"}`}
          >
            <div className="text-sm">General</div>
            <div className={`text-xs ${selectedId === GENERAL ? "text-white/80" : "text-muted-foreground"}`}>Paste the meeting context yourself</div>
          </button>
          {selectedId === GENERAL && (
            <textarea
              value={context}
              onChange={(event) => setContext(event.target.value)}
              rows={7}
              placeholder={"Agenda, names, decisions, or notes for this meeting.\nThe assistant answers from this text."}
              className="w-full border border-border bg-background px-2 py-2 text-xs text-foreground"
            />
          )}
          {projects.map((project) => {
            const selected = project.id === selectedId;
            return (
              <button
                key={project.id}
                onClick={() => setSelectedId(project.id)}
                className={`block w-full border px-3 py-2 text-left cursor-pointer ${selected ? "border-primary bg-primary text-primary-foreground" : "border-border text-foreground"}`}
              >
                <div className="text-sm">{project.name}</div>
                <div className={`truncate text-xs ${selected ? "text-white/80" : "text-muted-foreground"}`}>{project.root_path}</div>
                <div className={`text-xs ${selected ? "text-white/80" : "text-muted-foreground"}`}>
                  {project.scanned_at ? "Index available · check freshness below" : "No source index yet"}
                </div>
              </button>
            );
          })}
        </div>
        <ReadinessChecks key={selectedId} projectId={selectedId === GENERAL ? undefined : selectedId} />
        {error && <p role="alert" className="px-3 text-xs text-destructive">{error}</p>}
        <div className="flex gap-2 border-t border-border px-3 py-3">
          <button
            onClick={start}
            disabled={starting}
            className="inline-flex flex-1 items-center justify-center gap-2 bg-primary px-3 py-2 text-sm text-white disabled:opacity-50 cursor-pointer"
          >
            <Play className="h-4 w-4" />
            {starting ? "Starting" : "Start meeting"}
          </button>
          <button onClick={onCancel} className="border border-border px-3 py-2 text-sm text-foreground cursor-pointer">
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
