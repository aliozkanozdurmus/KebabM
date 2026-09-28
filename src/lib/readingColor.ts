/** Treat the old fixed defaults as automatic, without rewriting saved custom colors. */
export function readingColor(value: string, kind: 'answer' | 'transcript' | 'translation'): string {
  const legacy = { answer: '#d4d4d8', transcript: '#e4e4e7', translation: '#fbbf24' };
  if (!value || value === 'auto' || value.toLowerCase() === legacy[kind]) {
    return kind === 'translation' ? 'hsl(var(--reading-translation))' : 'hsl(var(--foreground))';
  }
  return value;
}
