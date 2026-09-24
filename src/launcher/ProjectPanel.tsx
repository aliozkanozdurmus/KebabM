import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderPlus, RefreshCw, Trash2 } from "lucide-react";
import {
  createProject,
  deleteProject,
  listProjects,
  importMeetingTranscript,
  scanProject,
  setActiveProject,
  type ProjectRecord,
} from "../lib/ipc";
import { showToast } from "../stores/toastStore";
import type { MeetingSummary } from "../lib/types";
import { formatRelativeTime } from "../lib/utils";

export function ProjectPanel({
  meetings,
  selectedMeetingId,
  focusProjectId,
  onSelectMeeting,
  onFocusProject,
  onProjects,
  onChanged,
}: {
  meetings: MeetingSummary[];
  selectedMeetingId?: string | null;
  focusProjectId: string | null;
  onSelectMeeting: (meetingId: string) => void;
  onFocusProject: (projectId: string | null) => void;
  onProjects?: (projects: ProjectRecord[]) => void;
  onChanged?: () => void;
}) {
  const onProjectsRef = useRef(onProjects);
  onProjectsRef.current = onProjects;
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [scanningId, setScanningId] = useState<string | null>(null);
  const [scanDetail, setScanDetail] = useState("");
  const [pasteFor, setPasteFor] = useState<string | null>(null);
  const [pasteTitle, setPasteTitle] = useState("");
  const [pasteText, setPasteText] = useState("");
  const [savingPaste, setSavingPaste] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const rows = await listProjects();
      setProjects(rows);
      onProjectsRef.current?.(rows);
      return rows;
    } catch (err) {
      showToast(err instanceof Error ? err.message : "Could not load projects", "error");
      return [];
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const addProject = async () => {
    const selected = await open({ directory: true, title: "Choose the project folder" });
    if (typeof selected !== "string") return;
    const name = selected.split(/[/\\]/).filter(Boolean).pop() || "Project";
    let createdId: string | null = null;
    try {
      const project = await createProject(name, selected);
      createdId = project.id;
      setProjects((current) => current.some((item) => item.id === project.id) ? current : [...current, project]);
      onFocusProject(project.id);
      setScanningId(project.id);
      setScanDetail("Reading the project");
      const unlisten = await listen<{ detail: string }>("project-scan", (event) => {
        setScanDetail(event.payload.detail);
      });
      try {
        await scanProject(project.id);
        showToast(`${name} is ready for meetings`, "success");
      } catch (err) {
        showToast(err instanceof Error ? err.message : "The project was added. The knowledge base was not written.", "error");
      } finally {
        unlisten();
      }
      await setActiveProject(project.id);
    } catch (err) {
      showToast(err instanceof Error ? err.message : "Could not add the project", "error");
    } finally {
      setScanningId(null);
      setScanDetail("");
      await refresh();
      if (createdId) onFocusProject(createdId);
      onChanged?.();
    }
  };

  const unassigned = meetings.filter((meeting) => !meeting.project_id);

  return (
    <aside className="flex h-full w-[280px] shrink-0 flex-col border-r border-border bg-card text-foreground">
      <div className="flex items-center justify-between px-3 py-3">
        <h2 className="text-xs uppercase tracking-wider text-muted-foreground">Projects</h2>
        <button onClick={addProject} className="inline-flex items-center gap-1 text-xs text-[#78a9ff] cursor-pointer" aria-label="Add project folder">
          <FolderPlus className="h-3.5 w-3.5" />
          Add
        </button>
      </div>
      <div className="flex-1 overflow-y-auto px-2 pb-3">
        <button
          onClick={() => onFocusProject(null)}
          className={`mb-2 block w-full px-2 py-2 text-left text-sm cursor-pointer ${focusProjectId === null ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-accent"}`}
        >
          All projects
        </button>
        {projects.length === 0 && (
          <p className="px-2 py-2 text-xs text-muted-foreground">Add a folder. Meetings for that project show under its name.</p>
        )}
        {projects.map((project) => {
          const projectMeetings = meetings.filter((meeting) => meeting.project_id === project.id);
          const focused = focusProjectId === project.id;
          return (
            <div key={project.id} className="mb-2">
              <button
                className={`block w-full px-2 py-2 text-left cursor-pointer ${focused ? "bg-primary text-primary-foreground" : "hover:bg-accent"}`}
                onClick={async () => {
                  await setActiveProject(project.id);
                  onFocusProject(project.id);
                  await refresh();
                  onChanged?.();
                }}
              >
                <div className="truncate text-sm">{project.name}</div>
                <div className={`text-[11px] ${focused ? "text-primary-foreground/80" : "text-muted-foreground"}`}>
                  {project.brief ? "Knowledge base ready" : "No knowledge base yet"}
                  {" · "}
                  {projectMeetings.length} meeting{projectMeetings.length === 1 ? "" : "s"}
                </div>
              </button>
              <div className="mt-1 space-y-0.5 pl-2">
                {projectMeetings.length === 0 && (
                  <p className="px-2 py-1 text-[11px] text-muted-foreground">No meetings yet</p>
                )}
                {projectMeetings.map((meeting) => (
                  <button
                    key={meeting.id}
                    onClick={() => onSelectMeeting(meeting.id)}
                    className={`block w-full px-2 py-1.5 text-left cursor-pointer ${selectedMeetingId === meeting.id ? "bg-accent" : "hover:bg-accent"}`}
                  >
                    <div className="truncate text-xs text-foreground">{meeting.title || "Untitled meeting"}</div>
                    <div className="text-[11px] text-muted-foreground">{formatRelativeTime(meeting.start_time)}</div>
                  </button>
                ))}
              </div>
              <div className="mt-1 flex flex-wrap gap-2 px-2">
                <button
                  className="inline-flex items-center gap-1 text-[11px] text-muted-foreground cursor-pointer"
                  disabled={scanningId === project.id}
                  onClick={async () => {
                    setScanningId(project.id);
                    setScanDetail("Starting");
                    const unlisten = await listen<{ detail: string }>("project-scan", (event) => {
                      setScanDetail(event.payload.detail);
                    });
                    try {
                      await scanProject(project.id);
                      await setActiveProject(project.id);
                      onFocusProject(project.id);
                      showToast(`${project.name} knowledge base is ready`, "success");
                      await refresh();
                      onChanged?.();
                    } catch (err) {
                      showToast(err instanceof Error ? err.message : "Scan failed", "error");
                    } finally {
                      unlisten();
                      setScanningId(null);
                      setScanDetail("");
                    }
                  }}
                >
                  <RefreshCw className={`h-3 w-3 ${scanningId === project.id ? "animate-spin" : ""}`} />
                  {scanningId === project.id ? scanDetail || "Reading" : "Build knowledge"}
                </button>
                <button
                  className="text-[11px] text-muted-foreground cursor-pointer"
                  onClick={() => {
                    setPasteFor(pasteFor === project.id ? null : project.id);
                    setPasteTitle("");
                    setPasteText("");
                  }}
                >
                  Add transcript
                </button>
                <button
                  className="inline-flex items-center gap-1 text-[11px] text-[#fa4d56] cursor-pointer"
                  onClick={async () => {
                    await deleteProject(project.id);
                    if (focusProjectId === project.id) onFocusProject(null);
                    await refresh();
                    onChanged?.();
                  }}
                  aria-label={`Remove ${project.name}`}
                >
                  <Trash2 className="h-3 w-3" />
                  Remove
                </button>
              </div>
              {pasteFor === project.id && (
                <form
                  className="mt-2 space-y-2 px-2"
                  onSubmit={async (event) => {
                    event.preventDefault();
                    if (!pasteTitle.trim() || !pasteText.trim()) return;
                    setSavingPaste(true);
                    try {
                      await importMeetingTranscript(project.id, pasteTitle.trim(), pasteText);
                      setPasteFor(null);
                      setPasteTitle("");
                      setPasteText("");
                      showToast("Transcript added to the project", "success");
                      onChanged?.();
                    } catch (err) {
                      showToast(err instanceof Error ? err.message : "Could not save the transcript", "error");
                    } finally {
                      setSavingPaste(false);
                    }
                  }}
                >
                  <input
                    value={pasteTitle}
                    onChange={(e) => setPasteTitle(e.target.value)}
                    placeholder="Meeting title"
                    className="w-full border-b-2 border-border bg-background px-2 py-1.5 text-xs text-foreground"
                  />
                  <textarea
                    value={pasteText}
                    onChange={(e) => setPasteText(e.target.value)}
                    placeholder={"Ali: We ship the login on Friday.\n00:12 Sam: Who owns the API?"}
                    rows={5}
                    className="w-full border-b-2 border-border bg-background px-2 py-1.5 text-xs text-foreground"
                  />
                  <button
                    type="submit"
                    disabled={savingPaste}
                    className="bg-primary px-3 py-1.5 text-xs text-primary-foreground disabled:opacity-50 cursor-pointer"
                  >
                    {savingPaste ? "Saving" : "Save transcript"}
                  </button>
                </form>
              )}
            </div>
          );
        })}
        <div className="mt-3 border-t border-border pt-3">
          <button
            onClick={() => onFocusProject("unassigned")}
            className={`block w-full px-2 py-2 text-left text-sm cursor-pointer ${focusProjectId === "unassigned" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-accent"}`}
          >
            No project
            <span className="ml-2 text-[11px]">{unassigned.length}</span>
          </button>
          <div className="mt-1 space-y-0.5 pl-2">
            {unassigned.map((meeting) => (
              <button
                key={meeting.id}
                onClick={() => onSelectMeeting(meeting.id)}
                className={`block w-full px-2 py-1.5 text-left cursor-pointer ${selectedMeetingId === meeting.id ? "bg-accent" : "hover:bg-accent"}`}
              >
                <div className="truncate text-xs text-foreground">{meeting.title || "Untitled meeting"}</div>
                <div className="text-[11px] text-muted-foreground">{formatRelativeTime(meeting.start_time)}</div>
              </button>
            ))}
          </div>
        </div>
      </div>
    </aside>
  );
}
