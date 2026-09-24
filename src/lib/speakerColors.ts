// Speaker color palette — 8 distinct colors for diarized speakers
// "you" and "them" use existing colors; diarized speakers assigned in order

export const SPEAKER_COLORS = [
  "#0f62fe",
  "#0043ce",
  "#78a9ff",
  "#24a148",
  "#f1c21b",
  "#da1e28",
  "#525252",
  "#002d9c",
] as const;

export const FIXED_SPEAKER_COLORS: Record<string, string> = {
  you: "#0f62fe",
  them: "#525252",
  room: "#161616",
};

export function getSpeakerColor(speakerId: string, orderIndex: number): string {
  if (speakerId in FIXED_SPEAKER_COLORS) {
    return FIXED_SPEAKER_COLORS[speakerId];
  }
  return SPEAKER_COLORS[orderIndex % SPEAKER_COLORS.length];
}

// Badge colors for audio mode
export const MODE_COLORS = {
  online: { text: "#0f62fe", bg: "#edf5ff" },
  in_person: { text: "#161616", bg: "#e0e0e0" },
} as const;
