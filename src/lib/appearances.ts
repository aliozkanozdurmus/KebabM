/** App-owned interpretations of the Open Design references; see docs/design/README.md. */
export type AppearanceId = 'ibm' | 'liquid-glass' | 'apple' | 'linear' | 'notion' | 'material' | 'github' | 'terminal';
type Palette = readonly [background: string, surface: string, foreground: string, muted: string, border: string, accent: string, onAccent: string];
interface Appearance {
  id: AppearanceId;
  name: string;
  description: string;
  font: string;
  radius: number;
  light: Palette;
  dark: Palette;
}
const system = '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const mono = '"IBM Plex Mono", Menlo, Consolas, monospace';
export const appearances: readonly Appearance[] = [
  { id: 'ibm', name: 'IBM', description: 'Carbon precision. Square edges, Plex type.', font: '"IBM Plex Sans", sans-serif', radius: 0,
    light: ['#ffffff','#f4f4f4','#161616','#525252','#c6c6c6','#0f62fe','#ffffff'], dark: ['#161616','#262626','#f4f4f4','#c6c6c6','#393939','#0f62fe','#ffffff'] },
  { id: 'liquid-glass', name: 'Liquid Glass', description: 'Translucent layers. Soft light, clear answers.', font: system, radius: 18,
    light: ['#e6edf4','#f7fafc','#15263b','#495d73','#aebdcd','#0062cc','#ffffff'], dark: ['#101d2c','#1e3146','#f1f7ff','#b6c9df','#45607b','#8ac3ff','#08294e'] },
  { id: 'apple', name: 'Apple', description: 'Native simplicity. Graphite and soft corners.', font: system, radius: 10,
    light: ['#f5f5f7','#ffffff','#1d1d1f','#626268','#d2d2d7','#0071e3','#ffffff'], dark: ['#161617','#272729','#f5f5f7','#b6b6bd','#454548','#70b7ff','#072a50'] },
  { id: 'linear', name: 'Linear', description: 'Focused workspace. Fine borders, indigo accents.', font: system, radius: 6,
    light: ['#fafafa','#ffffff','#202127','#62636c','#d5d6de','#5558bd','#ffffff'], dark: ['#0f1011','#191a1d','#f7f8f8','#aaaeb8','#383a43','#a4a6ff','#181937'] },
  { id: 'notion', name: 'Notion', description: 'Warm paper. Editorial headings, quiet surfaces.', font: system, radius: 3,
    light: ['#f6f5f4','#ffffff','#31302e','#615d59','#d4d0cb','#0969bd','#ffffff'], dark: ['#242321','#31302e','#f3f0eb','#bfb9b0','#504c46','#88bbdf','#172c3b'] },
  { id: 'material', name: 'Material', description: 'Tonal surfaces. Generous, rounded controls.', font: '"Segoe UI", system-ui, sans-serif', radius: 22,
    light: ['#f8f5fc','#eee8f5','#241e30','#62596f','#c8bfd4','#6442d6','#ffffff'], dark: ['#1c1822','#2b2435','#f4eefa','#c9bfd5','#53475f','#c8b3fd','#302054'] },
  { id: 'github', name: 'GitHub', description: 'Code-first clarity. Crisp panels, familiar blues.', font: system, radius: 6,
    light: ['#ffffff','#f6f8fa','#1f2328','#656d76','#d0d7de','#0969da','#ffffff'], dark: ['#0d1117','#161b22','#e6edf3','#a5aeb9','#30363d','#79b8ff','#092749'] },
  { id: 'terminal', name: 'Terminal', description: 'Monospace focus. Sharp lines, green signals.', font: mono, radius: 0,
    light: ['#f1f5ee','#fbfdf8','#18261b','#4f6252','#bbcbbb','#246d39','#ffffff'], dark: ['#101710','#182219','#e0efe0','#a6bda7','#344d37','#8bd694','#142f1a'] },
];
export function isAppearanceId(value: unknown): value is AppearanceId {
  return appearances.some(a => a.id === value);
}
export function resolveAppearance(value: unknown): AppearanceId {
  return isAppearanceId(value) ? value : 'ibm';
}
function hsl(hex: string): string {
  const [r,g,b] = [1,3,5].map(i => parseInt(hex.slice(i,i+2),16)/255);
  const max = Math.max(r,g,b), min = Math.min(r,g,b), d = max-min, l = (max+min)/2;
  let h = 0;
  if (d) h = max === r ? ((g-b)/d+6)%6 : max === g ? (b-r)/d+2 : (r-g)/d+4;
  return `${(h*60).toFixed(2)} ${(d ? d/(1-Math.abs(2*l-1))*100 : 0).toFixed(2)}% ${(l*100).toFixed(2)}%`;
}
export function appearanceVariables(id: AppearanceId, mode: 'light' | 'dark'): Record<string,string> {
  if (id === 'ibm') return {}; // Keep the original Carbon theme exactly as shipped.
  const a = appearances.find(a => a.id === id)!;
  const [background, surface, foreground, muted, border, primary, onPrimary] = a[mode];
  const colors: Record<string,string> = {
    background, foreground, card: surface, 'card-foreground': foreground, popover: surface, 'popover-foreground': foreground,
    primary, 'primary-foreground': onPrimary, secondary: surface, 'secondary-foreground': foreground,
    muted: surface, 'muted-foreground': muted, accent: border, 'accent-foreground': foreground, border, input: surface, ring: primary,
    info: primary, 'info-foreground': onPrimary, 'speaker-interviewer': primary,
    success: mode === 'dark' ? '#8bd694' : '#246d39', warning: mode === 'dark' ? '#f4c56a' : '#846000',
    'reading-translation': mode === 'dark' ? '#f4c56a' : '#846000',
    destructive: mode === 'dark' ? '#ff9299' : '#b42336',
  };
  const vars: Record<string,string> = Object.fromEntries(Object.entries(colors).map(([k,v])=>[`--${k}`,hsl(v)]));
  Object.assign(vars, {
    '--appearance-font-sans': a.font, '--appearance-font-mono': mono,
    '--appearance-heading-font': id === 'notion' ? 'Georgia, "Times New Roman", serif' : a.font,
    '--appearance-heading-weight': id === 'terminal' ? '500' : '600',
    '--appearance-shadow': id === 'liquid-glass' ? '0 8px 28px #061b3520, inset 0 1px 0 #ffffff40' : id === 'apple' || id === 'material' ? '0 2px 8px #00000012' : 'none',
  });
  for (const [size, scale] of Object.entries({ xs:.25, sm:.4, md:.6, lg:.8, xl:1, '2xl':1.2, '3xl':1.4, '4xl':1.6 })) vars[`--appearance-radius-${size}`] = `${a.radius*scale}px`;
  const carbon: Record<string,string> = { background, 'text-primary':foreground, 'text-secondary':muted, 'layer-01':surface,
    'layer-02':border, 'layer-hover':border, 'border-subtle':border, 'button-primary':primary,
    'button-primary-hover':primary, 'button-primary-active':primary, 'link-primary':primary, 'link-primary-hover':primary,
    focus:primary, 'focus-inset':surface, field:surface, 'support-info':primary, 'support-success':colors.success, 'support-error':colors.destructive, 'support-warning':colors.warning };
  for (const [key,value] of Object.entries(carbon)) vars[`--cds-${key}`] = value;
  return vars;
}
