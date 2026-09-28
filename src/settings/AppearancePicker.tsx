import { BrandMark } from "../components/BrandMark";
import type { CSSProperties } from 'react';
import { Check } from 'lucide-react';
import { appearances, appearanceVariables } from '../lib/appearances';
import { useConfigStore } from '../stores/configStore';
import { useEffect, useState } from 'react';

export function AppearancePicker() {
  const appearance = useConfigStore(s => s.appearance);
  const setAppearance = useConfigStore(s => s.setAppearance);
  const theme = useConfigStore(s => s.theme);
  const [systemDark, setSystemDark] = useState(() => matchMedia('(prefers-color-scheme: dark)').matches);
  useEffect(() => {
    const query = matchMedia('(prefers-color-scheme: dark)');
    const update = () => setSystemDark(query.matches);
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  const mode = theme === 'system' ? (systemDark ? 'dark' : 'light') : theme;
  return (
    <fieldset className="min-w-0">
      <legend className="text-base font-semibold text-foreground">Appearance</legend>
      <p className="mt-1 mb-4 text-xs leading-relaxed text-muted-foreground">
        Choose the workspace that feels right. Applies to every window, including live help.
      </p>
      <div className="appearance-grid">
        {appearances.map(a => {
          const [bg, surface, text, muted, border, accent, onAccent] = a[mode];
          const selected = appearance === a.id;
          return (
            <label key={a.id} className="appearance-option" data-selected={selected}>
              <input type="radio" name="appearance" value={a.id} checked={selected}
                onChange={() => setAppearance(a.id)} aria-label={a.name} className="appearance-radio" />
              <span aria-hidden="true" className="appearance-preview" style={{
                ...appearanceVariables(a.id, mode), background: bg, color: text, borderColor: border, fontFamily: a.font,
                borderRadius: a.radius * .55,
              } as CSSProperties}>
                <span className="appearance-preview-bar" style={{ borderColor: border, background: a.id === 'liquid-glass' ? `linear-gradient(110deg, ${surface}, ${border})` : surface }}>
                  <BrandMark decorative className="h-3.5 w-3.5" />
                  <span>KebabM</span><span className="ml-auto" style={{ color: muted }}>···</span>
                </span>
                <span className="appearance-preview-content">
                  <span style={{ color: muted }}>LIVE HELP</span>
                  <strong style={{ fontFamily: a.id === 'notion' ? 'Georgia, serif' : a.font }}>An answer, ready.</strong>
                  <span className="appearance-preview-line" style={{ background: border }} />
                  <span className="appearance-preview-action" style={{ background: accent, color: onAccent, borderRadius: a.radius * .5 }}>View sources <span>↗</span></span>
                </span>
              </span>
              <span className="flex items-center justify-between gap-2 text-sm font-medium">
                {a.name}<Check className={`h-4 w-4 shrink-0 text-primary ${selected ? '' : 'invisible'}`} aria-hidden="true" />
              </span>
              <span className="text-xs leading-relaxed text-muted-foreground">{a.description}</span>
            </label>
          );
        })}
      </div>
      <p className="mt-3 text-xs text-muted-foreground" aria-live="polite">
        {appearances.find(a => a.id === appearance)?.name} selected. Changes save automatically.
      </p>
    </fieldset>
  );
}
