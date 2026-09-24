export function splitFollowUps(markdown: string): { body: string; questions: string[] } {
  const parts = markdown.split(/\n## Follow-ups\s*\n/i);
  if (parts.length < 2) return { body: markdown, questions: [] };
  const questions = parts
    .slice(1)
    .join("\n")
    .split("\n")
    .map((line) => line.replace(/^[-*]\s*/, "").trim())
    .filter((line) => line.length > 8)
    .slice(0, 3);
  return { body: parts[0].trim(), questions };
}
