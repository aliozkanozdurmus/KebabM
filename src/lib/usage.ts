// Standard uncached Anthropic Sonnet 5 text rates, verified 2026-09-26.
// https://platform.claude.com/docs/en/models/sonnet-5/overview
// Unknown models/providers remain unpriced; this is never presented as a bill.
export function estimateAnswerCost(provider: string, model: string, input?: number, output?: number): number | undefined {
  if (provider.toLowerCase() !== "anthropic" || model !== "claude-sonnet-5" || input == null || output == null || input < 0 || output < 0 || !Number.isFinite(input + output) || input + output === 0) return undefined;
  return (input * 2 + output * 10) / 1_000_000;
}

export const costScope = "Estimated standard text-answer cost. Excludes cached-input charges, STT, translation, embeddings, question detection and preparation. Prices verified 2026-09-26; provider billing is authoritative.";
