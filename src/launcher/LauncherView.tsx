import { TodayView } from "./TodayView";
import type { CalendarEvent } from "../lib/calendar";
import "./workspace.css";
import { BrandMark } from "../components/BrandMark";
import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { useMeetingStore } from "../stores/meetingStore";
import { useConfigStore } from "../stores/configStore";
import { useContextStore } from "../stores/contextStore";
import { useRagStore } from "../stores/ragStore";
import { searchMeetings, deleteMeeting, getPlatformCapabilities, setMeetingFeed, type ProjectRecord } from "../lib/ipc";
import { showToast } from "../stores/toastStore";
import { RecentMeetings } from "./RecentMeetings";
import { MeetingDetails } from "./meeting-details";
import { MeetingSetupModal } from "./MeetingSetupModal";
import { ProjectWorkspace } from "./ProjectWorkspace";
import { ProjectPanel } from "./ProjectPanel";
import { FileUpload } from "../context/FileUpload";
import { ResourceCard } from "../context/ResourceCard";
import { TokenBudget } from "../context/TokenBudget";
import { TestSearchDialog } from "../context/TestSearchDialog";
import { NEXQ_VERSION, NEXQ_DEVELOPER } from "../lib/version";
import { ServiceStatusBar } from "../components/ServiceStatusBar";
import { showOverlayWindow } from "../lib/windows";
import type { MeetingSummary, AudioMode, AIScenario, PlatformCapabilities } from "../lib/types";
import {
  Settings,
  Search,
  AlertTriangle,
  X,
  Loader2,
  Star,
  Trash2,
  Database,
  Zap,
  CheckCircle2,
  ArrowRight,
  Radio,
  Play,
  FlaskConical,
} from "lucide-react";

// ── Favorites (localStorage) ──

function useFavorites() {
  const [favorites, setFavorites] = useState<Set<string>>(() => {
    try {
      const stored = localStorage.getItem("nexq_favorites");
      return stored ? new Set(JSON.parse(stored)) : new Set();
    } catch {
      return new Set();
    }
  });
  const toggleFavorite = useCallback((id: string) => {
    setFavorites((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      localStorage.setItem("nexq_favorites", JSON.stringify([...next]));
      return next;
    });
  }, []);
  return { favorites, toggleFavorite };
}

type MeetingFilter = "all" | "favorites" | "with_summary" | "online" | "in_person";

// ════════════════════════════════════════════════════════════════
//  NEXQ DASHBOARD
// ════════════════════════════════════════════════════════════════

export function LauncherView() {
  const setCurrentView = useMeetingStore((s) => s.setCurrentView);
  const recentMeetings = useMeetingStore((s) => s.recentMeetings);
  const loadRecentMeetings = useMeetingStore((s) => s.loadRecentMeetings);
  const startMeetingFlow = useMeetingStore((s) => s.startMeetingFlow);
  const endMeetingFlow = useMeetingStore((s) => s.endMeetingFlow);
  const activeMeeting = useMeetingStore((s) => s.activeMeeting);

  const resources = useContextStore((s) => s.resources);
  const removeFile = useContextStore((s) => s.removeFile);
  const loadResources = useContextStore((s) => s.loadResources);
  const refreshTokenBudget = useContextStore((s) => s.refreshTokenBudget);

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<MeetingSummary[] | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);
  const selectedMeetingId = useMeetingStore((s) => s.selectedMeetingId);
  const setSelectedMeetingId = useMeetingStore((s) => s.setSelectedMeetingId);
  const [filter, setFilter] = useState<MeetingFilter>("all");
  const [showConflictPrompt, setShowConflictPrompt] = useState(false);
  const [showDeleteAllConfirm, setShowDeleteAllConfirm] = useState(false);
  const [isDeletingAll, setIsDeletingAll] = useState(false);
  const [ragStatus, setRagStatus] = useState<"idle" | "updating" | "done">("idle");
  const [showTestKB, setShowTestKB] = useState(false);
  const [showMeetingSetup, setShowMeetingSetup] = useState(false);
  const [platform, setPlatform] = useState<PlatformCapabilities | null>(null);
  const [workspaceTab, setWorkspaceTab] = useState<"today" | "prepare" | "history">("today");
  const [calendarEvent, setCalendarEvent] = useState<CalendarEvent | null>(null);
  const [focusProjectId, setFocusProjectId] = useState<string | null>(null);
  const [projects, setProjects] = useState<ProjectRecord[]>([]);
  const [generalContext, setGeneralContext] = useState<string | null>(null);
  // Pending audioMode/scenario from setup modal — used when conflict resolution triggers start
  const pendingMeetingSetup = useRef<{ audioMode: AudioMode; scenario: AIScenario } | null>(null);

  const contextStrategy = useConfigStore((s) => s.contextStrategy);
  const indexStatus = useRagStore((s) => s.indexStatus);
  const isIndexing = useRagStore((s) => s.isIndexing);
  const indexStale = useRagStore((s) => s.indexStale);
  const isAutoIndexing = useRagStore((s) => s.isAutoIndexing);
  const refreshIndexStatus = useRagStore((s) => s.refreshIndexStatus);
  const rebuildIndex = useRagStore((s) => s.rebuildIndex);
  const autoRemoveFileIndex = useRagStore((s) => s.autoRemoveFileIndex);

  const { favorites, toggleFavorite } = useFavorites();
  const autoStartTriggered = useRef(false);

  useEffect(() => {
    loadRecentMeetings();
    loadResources();
    refreshTokenBudget();
    refreshIndexStatus();
    getPlatformCapabilities().then(setPlatform).catch(() => {});
    if (!autoStartTriggered.current) {
      autoStartTriggered.current = true;
      const { startOnLogin } = useConfigStore.getState();
      const { activeMeeting: am } = useMeetingStore.getState();
      if (startOnLogin && !am) startMeetingFlow();
    }
  }, [loadRecentMeetings, startMeetingFlow, loadResources, refreshTokenBudget, refreshIndexStatus]);

  // ── Handlers ──

  // Called when user clicks Start Meeting button — open setup modal
  const handleStartMeeting = useCallback(() => {
    if (activeMeeting) { setShowConflictPrompt(true); return; }
    setCalendarEvent(null);
    setShowMeetingSetup(true);
  }, [activeMeeting]);

  // Called when user confirms setup in the modal
  const handleSetupConfirm = useCallback(async (choice: { projectId: string } | { context: string }) => {
    setShowMeetingSetup(false);
    setIsStarting(true);
    setStartError(null);
    try {
      if ("context" in choice) {
        setGeneralContext(choice.context);
        await setMeetingFeed([calendarEvent ? `Scheduled meeting: ${calendarEvent.title}\nTime: ${calendarEvent.start}` : "", choice.context].filter(Boolean).join("\n\n"));
      } else {
        setGeneralContext(null);
        await setMeetingFeed(calendarEvent ? `Scheduled meeting: ${calendarEvent.title}\nTime: ${calendarEvent.start}` : "");
      }
      await startMeetingFlow(calendarEvent?.title, "online", "team_meeting");
      showToast("Meeting started", "success");
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to start meeting";
      setStartError(msg);
      showToast(msg, "error");
    } finally {
      setIsStarting(false);
    }
  }, [startMeetingFlow, calendarEvent]);

  const handleEndAndStartNew = useCallback(async () => {
    setShowConflictPrompt(false);
    setIsStarting(true);
    const setup = pendingMeetingSetup.current;
    pendingMeetingSetup.current = null;
    try {
      await endMeetingFlow();
      await startMeetingFlow(undefined, setup?.audioMode, setup?.scenario);
      showToast("New meeting started", "success");
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to start";
      setStartError(msg);
    } finally {
      setIsStarting(false);
    }
  }, [endMeetingFlow, startMeetingFlow]);

  const handleSearch = useCallback(async (query: string) => {
    setSearchQuery(query);
    if (!query.trim()) { setSearchResults(null); return; }
    try { setSearchResults(await searchMeetings(query)); } catch { setSearchResults(null); }
  }, []);

  const handleDeleteMeeting = useCallback(async (meetingId: string) => {
    try {
      await deleteMeeting(meetingId);
      await loadRecentMeetings();
      if (searchResults) setSearchResults((p) => p?.filter((m) => m.id !== meetingId) ?? null);
      showToast("Meeting deleted", "info");
    } catch { showToast("Couldn't delete meeting", "error"); }
  }, [loadRecentMeetings, searchResults]);

  const handleDeleteAll = useCallback(() => {
    setShowDeleteAllConfirm(true);
  }, []);

  const handleConfirmDeleteAll = useCallback(async () => {
    setIsDeletingAll(true);
    try {
      for (const m of recentMeetings) { try { await deleteMeeting(m.id); } catch {} }
      await loadRecentMeetings();
      showToast("All meetings deleted", "info");
    } finally {
      setIsDeletingAll(false);
      setShowDeleteAllConfirm(false);
    }
  }, [recentMeetings, loadRecentMeetings]);

  const handleRenameMeeting = useCallback(async () => { await loadRecentMeetings(); }, [loadRecentMeetings]);

  const handleSelectMeeting = useCallback((meetingId: string) => {
    if (activeMeeting?.id === meetingId) {
      setCurrentView("overlay");
      showOverlayWindow().catch(() => {});
      return;
    }
    setSelectedMeetingId(meetingId);
  }, [activeMeeting, setCurrentView]);

  const handleReturnToMeeting = useCallback(() => {
    setCurrentView("overlay");
    showOverlayWindow().catch(() => {});
  }, [setCurrentView]);

  const handleRagUpdate = useCallback(async () => {
    setRagStatus("updating");
    try {
      await rebuildIndex();
      setRagStatus("done");
      setTimeout(() => setRagStatus("idle"), 3000);
    } catch (e) {
      setRagStatus("idle");
    }
  }, [rebuildIndex]);

  // ── Filtered meetings ──

  const displayedMeetings = useMemo(() => {
    let list = searchResults ?? recentMeetings;
    if (filter === "favorites") list = list.filter((m) => favorites.has(m.id));
    if (filter === "with_summary") list = list.filter((m) => m.has_summary);
    if (filter === "online") list = list.filter((m) => m.audio_mode === "online");
    if (filter === "in_person") list = list.filter((m) => m.audio_mode === "in_person");
    if (focusProjectId === "unassigned") list = list.filter((m) => !m.project_id);
    else if (focusProjectId) list = list.filter((m) => m.project_id === focusProjectId);
    return list;
  }, [searchResults, recentMeetings, filter, favorites, focusProjectId]);

  const visibleProjects = useMemo(() => {
    if (focusProjectId === "unassigned") return [];
    if (!focusProjectId) return projects;
    return projects.filter((project) => project.id === focusProjectId);
  }, [projects, focusProjectId]);

  // ── Meeting Details view ──

  if (selectedMeetingId) {
    return (
      <div className="app-surface flex h-full flex-col bg-background">
        <MeetingDetails meetingId={selectedMeetingId} onBack={() => setSelectedMeetingId(null)} />
      </div>
    );
  }

  return (
    <div className="app-surface meeting-workspace flex h-full flex-col bg-background">
      {/* ═══ HEADER ═══ */}
      <header className="dash-header flex min-h-12 shrink-0 flex-wrap items-center justify-between gap-2 border-b border-border bg-card px-4 text-foreground">
        <div className="flex items-center gap-3">
          <BrandMark decorative className="h-8 w-8" />
          <span className="text-sm font-medium tracking-normal">KebabM</span>
        </div>

        {/* Active meeting in header */}
        {activeMeeting && (
          <button
            onClick={handleReturnToMeeting}
            className="group live-ring-pulse flex items-center gap-2 rounded-full border border-success/20 bg-success/10 pl-3 pr-2 py-1.5 shadow-sm shadow-success/10 transition-all hover:bg-success/20 hover:border-success/30 hover:shadow-md hover:shadow-success/10 cursor-pointer"
          >
            <Radio className="h-3 w-3 text-success animate-pulse" />
            <span className="text-xs font-medium text-success max-w-[200px] truncate">
              {activeMeeting.title}
            </span>
            <span className="flex items-center gap-0.5 rounded-full bg-success/20 px-1.5 py-0.5 text-meta font-semibold text-success">
              RETURN <ArrowRight className="h-2.5 w-2.5" />
            </span>
          </button>
        )}

        <div className="flex items-center gap-2">
          <button
            onClick={handleStartMeeting}
            disabled={isStarting}
            aria-busy={isStarting}
            className="flex h-8 items-center gap-2 bg-primary px-4 text-sm font-normal text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-60 cursor-pointer"
          >
            {isStarting ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Play className="h-3.5 w-3.5" fill="currentColor" />}
            {isStarting ? "Starting" : "Start meeting"}
          </button>
          <button
            onClick={() => setCurrentView("settings")}
            className="p-2 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground cursor-pointer"
            aria-label="Settings"
          >
            <Settings className="h-4 w-4" aria-hidden="true" />
          </button>
        </div>
      </header>

      {/* ═══ MAIN DASHBOARD ═══ */}
      <div className="flex flex-1 overflow-hidden">
        {workspaceTab === "prepare" && <ProjectPanel
          meetings={searchResults ?? recentMeetings}
          selectedMeetingId={selectedMeetingId}
          focusProjectId={focusProjectId}
          onSelectMeeting={handleSelectMeeting}
          onFocusProject={(id) => { setFocusProjectId(id); setWorkspaceTab("prepare"); }}
          onProjects={setProjects}
          onChanged={() => {
            loadRecentMeetings();
          }}
        />}

        <main className="flex min-w-0 flex-1 flex-col">
          <nav className="workspace-navigation flex shrink-0 gap-4 border-b border-border px-4" aria-label="Workspace">
            <button aria-current={workspaceTab === "today" ? "page" : undefined} className={`py-3 text-sm border-b-2 ${workspaceTab === "today" ? "border-primary" : "border-transparent text-muted-foreground"}`} onClick={() => setWorkspaceTab("today")}>Today</button>
            <button className={`py-3 text-sm border-b-2 ${workspaceTab === "prepare" ? "border-primary" : "border-transparent text-muted-foreground"}`} onClick={() => setWorkspaceTab("prepare")}>Projects</button>
            <button className={`py-3 text-sm border-b-2 ${workspaceTab === "history" ? "border-primary" : "border-transparent text-muted-foreground"}`} onClick={() => setWorkspaceTab("history")}>All notes</button>
          </nav>
          {startError && <p role="alert" className="px-4 py-2 text-sm text-destructive">{startError}</p>}
          {workspaceTab === "today" ? <TodayView meetings={recentMeetings} onSelectMeeting={handleSelectMeeting} onHistory={() => setWorkspaceTab("history")} onStartEvent={event => { if (activeMeeting) { setShowConflictPrompt(true); return; } setCalendarEvent(event); setShowMeetingSetup(true); }} /> : workspaceTab === "prepare" ? <ProjectWorkspace key={focusProjectId ?? "active"} project={projects.find(p => p.id === focusProjectId) ?? (focusProjectId !== "unassigned" ? projects.find(p => p.is_active) : undefined)} onChanged={() => { loadRecentMeetings(); }} /> : <>
        {/* ── MEETINGS, GROUPED BY PROJECT ── */}
        <div className="dash-main flex min-w-0 flex-1 flex-col bg-background">
          {/* Search */}
          <div className="px-3 pt-3 pb-2">
            <div className="group relative">
              <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground/40 transition-colors group-focus-within:text-primary/70" aria-hidden="true" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => handleSearch(e.target.value)}
                placeholder="Search meetings..."
                aria-label="Search meetings"
                className="w-full rounded-lg border border-border/20 bg-background/40 py-2 pl-9 pr-8 text-sm text-foreground placeholder:text-muted-foreground/40 transition-all focus:border-primary/40 focus:bg-background/60 focus:outline-none focus:ring-1 focus:ring-primary/20"
              />
              {searchQuery && (
                <button onClick={() => handleSearch("")} className="absolute right-2.5 top-1/2 -translate-y-1/2 rounded-sm p-0.5 text-muted-foreground/40 transition-colors hover:text-foreground cursor-pointer" aria-label="Clear search">
                  <X className="h-3 w-3" aria-hidden="true" />
                </button>
              )}
            </div>
          </div>

          {/* Filters + Delete all */}
          <div className="flex items-center justify-between px-3 pb-2">
            <div className="flex items-center gap-0.5 flex-wrap">
              {([
                { key: "all", label: "All" },
                { key: "favorites", label: "Starred" },
                { key: "with_summary", label: "Summary" },
                { key: "online", label: "Online" },
                { key: "in_person", label: "In-Person" },
              ] as const).map(({ key, label }) => (
                <button
                  key={key}
                  onClick={() => setFilter(key)}
                  className={`rounded px-1.5 py-0.5 text-meta font-medium transition-all duration-150 active:scale-90 cursor-pointer ${
                    filter === key
                      ? key === "online"
                        ? "bg-[rgba(74,108,247,0.15)] text-[#4a6cf7]"
                        : key === "in_person"
                          ? "bg-[rgba(168,85,247,0.15)] text-[#a855f7]"
                          : "bg-primary/10 text-primary"
                      : "text-muted-foreground/60 hover:text-muted-foreground hover:bg-accent/30"
                  }`}
                >
                  {key === "favorites" && <Star className="mr-0.5 inline h-2 w-2" />}
                  {label}
                </button>
              ))}
            </div>
            {recentMeetings.length > 0 && (
              <button onClick={handleDeleteAll} className="rounded p-1 text-muted-foreground/50 transition-all duration-150 hover:text-destructive hover:bg-destructive/10 active:scale-90 cursor-pointer" aria-label="Delete all meetings">
                <Trash2 className="h-3 w-3" aria-hidden="true" />
              </button>
            )}
          </div>

          {/* Count */}
          <div className="px-3 pb-1.5">
            <span className="text-meta font-semibold uppercase tracking-wider text-muted-foreground/60">
              {displayedMeetings.length} meeting{displayedMeetings.length !== 1 ? "s" : ""}
            </span>
          </div>

          {/* Scrollable meeting list */}
          <div className="flex-1 overflow-y-auto px-2 pb-2 scrollbar-thin scrollbar-thumb-border/20">
            <RecentMeetings
              meetings={displayedMeetings}
              projects={visibleProjects}
              onSelect={handleSelectMeeting}
              onDelete={handleDeleteMeeting}
              onRename={handleRenameMeeting}
              favorites={favorites}
              onToggleFavorite={toggleFavorite}
              activeMeetingId={activeMeeting?.id ?? null}
            />
          </div>
        </div>

        </>}
        </main>
      </div>

      {/* ═══ FOOTER ═══ */}
      <footer className="dash-footer flex items-center justify-between border-t border-border/20">
        <details className="service-details"><summary>Services</summary><ServiceStatusBar /></details>
        <div className="flex items-center gap-2 pr-5 text-xs text-muted-foreground/60">
          <span>&copy; {new Date().getFullYear()} {NEXQ_DEVELOPER}</span>
          <span className="text-muted-foreground/40">|</span>
          <span className="font-medium">v{NEXQ_VERSION}</span>
        </div>
      </footer>

      {/* ═══ MEETING SETUP MODAL ═══ */}
      <MeetingSetupModal
        open={showMeetingSetup}
        meetingTitle={calendarEvent?.title}
        onStart={handleSetupConfirm}
        onCancel={() => setShowMeetingSetup(false)}
      />

      {/* ═══ TEST KNOWLEDGE BASE MODAL ═══ */}
      <TestSearchDialog isOpen={showTestKB} onClose={() => setShowTestKB(false)} />

      {/* ═══ DELETE ALL CONFIRMATION ═══ */}
      {showDeleteAllConfirm && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true" aria-label="Delete all meetings confirmation">
          <div className="w-[380px] rounded-2xl border border-border/40 bg-card p-5 shadow-2xl">
            <div className="mb-1 flex items-center gap-2">
              <Trash2 className="h-4.5 w-4.5 text-destructive" aria-hidden="true" />
              <h3 className="text-sm font-semibold text-foreground">Delete All Meetings</h3>
            </div>
            <p className="mb-5 text-xs text-muted-foreground">
              This will permanently delete all <span className="font-semibold text-foreground">{recentMeetings.length}</span> meetings and their transcripts. This action cannot be undone.
            </p>
            <div className="flex flex-col gap-2">
              <button
                autoFocus
                onClick={handleConfirmDeleteAll}
                disabled={isDeletingAll}
                className="w-full rounded-xl bg-destructive px-4 py-2 text-xs font-medium text-white transition-all duration-150 hover:bg-destructive/90 hover:-translate-y-px active:translate-y-px active:scale-[0.98] disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:translate-y-0 disabled:active:scale-100 cursor-pointer"
              >
                {isDeletingAll ? (
                  <span className="flex items-center justify-center gap-2">
                    <Loader2 className="h-3 w-3 animate-spin" />
                    Deleting...
                  </span>
                ) : (
                  `Delete All ${recentMeetings.length} Meetings`
                )}
              </button>
              <button
                onClick={() => setShowDeleteAllConfirm(false)}
                disabled={isDeletingAll}
                className="w-full rounded-xl border border-border/40 bg-secondary/30 px-4 py-2 text-xs font-medium text-foreground transition-all duration-150 hover:bg-secondary/50 active:scale-[0.98] disabled:opacity-60 cursor-pointer"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ═══ CONFLICT MODAL ═══ */}
      {showConflictPrompt && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" role="dialog" aria-modal="true" aria-label="Meeting in progress conflict">
          <div className="w-[380px] rounded-2xl border border-border/40 bg-card p-5 shadow-2xl">
            <div className="mb-1 flex items-center gap-2">
              <AlertTriangle className="h-4.5 w-4.5 text-warning" aria-hidden="true" />
              <h3 className="text-sm font-semibold text-foreground">Meeting in Progress</h3>
            </div>
            <p className="mb-5 text-xs text-muted-foreground">
              &ldquo;{activeMeeting?.title}&rdquo; is still active.
            </p>
            <div className="flex flex-col gap-2">
              <button autoFocus onClick={handleEndAndStartNew} className="w-full rounded-xl bg-primary px-4 py-2 text-xs font-medium text-primary-foreground transition-all duration-150 hover:bg-primary/90 hover:-translate-y-px active:translate-y-px active:scale-[0.98] cursor-pointer">
                End Current & Start New
              </button>
              <button onClick={() => { setShowConflictPrompt(false); handleReturnToMeeting(); }} className="w-full rounded-xl border border-border/40 bg-secondary/30 px-4 py-2 text-xs font-medium text-foreground transition-all duration-150 hover:bg-secondary/50 active:scale-[0.98] cursor-pointer">
                Return to Current Meeting
              </button>
              <button onClick={() => setShowConflictPrompt(false)} className="w-full rounded-xl px-4 py-1.5 text-xs text-muted-foreground hover:text-foreground cursor-pointer">
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
