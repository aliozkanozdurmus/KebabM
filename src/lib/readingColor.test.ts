import { expect, it } from 'vitest';
import { readingColor } from './readingColor';
it('adapts legacy text defaults to the appearance without discarding custom colors', () => {
  expect(readingColor('#D4D4D8','answer')).toBe('hsl(var(--foreground))');
  expect(readingColor('auto','transcript')).toBe('hsl(var(--foreground))');
  expect(readingColor('#fbbf24','translation')).toBe('hsl(var(--reading-translation))');
  expect(readingColor('#abcdef','answer')).toBe('#abcdef');
});
