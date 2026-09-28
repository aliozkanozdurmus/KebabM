import { expect, it } from 'vitest';
import { appearances, appearanceVariables, resolveAppearance } from './appearances';
function luminance(hex: string) {
  const v = [1,3,5].map(i => parseInt(hex.slice(i,i+2),16)/255).map(c => c <= .04045 ? c/12.92 : ((c+.055)/1.055)**2.4);
  return v[0]*.2126 + v[1]*.7152 + v[2]*.0722;
}
function contrast(a:string,b:string) { const x=luminance(a),y=luminance(b); return (Math.max(x,y)+.05)/(Math.min(x,y)+.05); }
it('retains IBM for missing, removed or invalid preferences', () => {
  for (const value of [undefined, null, {}, '', 'future-theme']) expect(resolveAppearance(value)).toBe('ibm');
  expect(appearanceVariables('ibm','dark')).toEqual({});
});
for (const appearance of appearances) for (const mode of ['light','dark'] as const) {
  it(`${appearance.name} ${mode} keeps answer, secondary text and primary controls legible`, () => {
    const [bg,surface,text,muted,,accent,onAccent] = appearance[mode];
    for (const canvas of [bg,surface]) {
      expect(contrast(text,canvas)).toBeGreaterThanOrEqual(7);
      expect(contrast(muted,canvas)).toBeGreaterThanOrEqual(4.5);
    }
    expect(contrast(accent,onAccent)).toBeGreaterThanOrEqual(4.5);
  });
}
