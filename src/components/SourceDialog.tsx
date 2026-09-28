import { useEffect, useRef } from 'react';
import { X } from 'lucide-react';
import type { EvidenceRef } from '../lib/types';
export function SourceDialog({ source, onClose }: { source: { ref: EvidenceRef; text: string }; onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => { const previous = document.activeElement as HTMLElement | null; dialog.current?.showModal(); return () => { dialog.current?.close(); previous?.focus(); }; }, []);
  return <dialog ref={dialog} onCancel={onClose} onClick={e => { if (e.target === e.currentTarget) onClose(); }} aria-label="Source snapshot" className="m-auto max-h-[85vh] w-[calc(100%-2rem)] max-w-2xl overflow-y-auto border border-border bg-background p-4 text-foreground backdrop:bg-black/60">
    <div className="flex items-start gap-3"><h2 className="flex-1 break-all text-sm font-mono">{source.ref.path}:{source.ref.startLine}–{source.ref.endLine}</h2><button autoFocus aria-label="Close source" onClick={onClose}><X size={16} /></button></div>
    <p className="my-3 text-xs text-muted-foreground">Indexed snapshot · {source.ref.revision.slice(0, 10)}{source.ref.dirty ? ' · local changes' : ''}</p>
    <pre className="whitespace-pre-wrap break-words text-xs leading-relaxed">{source.text}</pre>
  </dialog>;
}
