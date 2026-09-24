import type { MeetingSummary } from "../lib/types";
import type { ProjectRecord } from "../lib/ipc";
import { MeetingCard } from "./MeetingCard";
import { Mic } from "lucide-react";

interface RecentMeetingsProps {
  meetings: MeetingSummary[];
  projects?: ProjectRecord[];
  onSelect: (meetingId: string) => void;
  onDelete: (meetingId: string) => void;
  onRename: (meetingId: string, newTitle: string) => void;
  favorites?: Set<string>;
  onToggleFavorite?: (meetingId: string) => void;
  activeMeetingId?: string | null;
}

const UNASSIGNED = "No project";

/** Groups meetings under their project. Meetings without a project come last. */
function groupMeetingsByProject(
  meetings: MeetingSummary[]
): Map<string, MeetingSummary[]> {
  const named = new Map<string, MeetingSummary[]>();
  const unassigned: MeetingSummary[] = [];

  for (const meeting of meetings) {
    const label = meeting.project_name?.trim();
    if (!label) {
      unassigned.push(meeting);
      continue;
    }
    const list = named.get(label) ?? [];
    list.push(meeting);
    named.set(label, list);
  }

  const groups = new Map(
    [...named.entries()].sort(([a], [b]) => a.localeCompare(b))
  );
  if (unassigned.length > 0) groups.set(UNASSIGNED, unassigned);
  return groups;
}

export function RecentMeetings({
  meetings,
  projects = [],
  onSelect,
  onDelete,
  onRename,
  favorites,
  onToggleFavorite,
  activeMeetingId,
}: RecentMeetingsProps) {
  const grouped = groupMeetingsByProject(meetings);
  for (const project of [...projects].sort((a, b) => a.name.localeCompare(b.name))) {
    if (!grouped.has(project.name)) grouped.set(project.name, []);
  }

  if (meetings.length === 0 && projects.length === 0) {
    return (
      <div className="dash-main flex flex-col items-center justify-center rounded-2xl border border-border/20 bg-secondary/10 py-14">
        <div className="mb-3 rounded-full bg-primary/10 p-3.5">
          <Mic className="h-4.5 w-4.5 text-primary/30" />
        </div>
        <p className="text-xs font-medium text-muted-foreground/50">
          No meetings yet
        </p>
        <p className="mt-1 text-meta text-muted-foreground/60">
          Add a project, then start a meeting
        </p>
      </div>
    );
  }

  // Running counter for staggered card entrance across all groups
  let cardIndex = 0;

  return (
    <div className="space-y-4">
      {Array.from(grouped.entries()).map(([dateGroup, groupMeetings]) => (
        <div key={dateGroup}>
          <h3 className="mb-2 text-meta font-semibold uppercase tracking-wider text-muted-foreground/60">
            {dateGroup}
          </h3>
          <div className="space-y-1.5">
            {groupMeetings.length === 0 && (
              <p className="whitespace-pre-wrap px-2 py-2 text-xs text-foreground/80">
                {(projects.find((project) => project.name === dateGroup)?.brief || "No meetings yet.")
                  .replace(/^#+\s*/gm, "")
                  .slice(0, 700)}
              </p>
            )}
            {groupMeetings.map((meeting) => {
              const idx = cardIndex++;
              return (
                <MeetingCard
                  key={meeting.id}
                  meeting={meeting}
                  onSelect={onSelect}
                  onDelete={onDelete}
                  onRename={onRename}
                  isFavorite={favorites?.has(meeting.id) ?? false}
                  onToggleFavorite={onToggleFavorite}
                  isLive={meeting.id === activeMeetingId}
                  staggerIndex={idx}
                />
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
}
