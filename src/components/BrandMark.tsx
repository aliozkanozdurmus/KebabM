export function BrandMark({ className = "h-7 w-7", decorative = false }: { className?: string; decorative?: boolean }) {
  return <img src="/brand/kebabm-mark.png" alt={decorative ? "" : "KebabM"} className={`shrink-0 object-contain ${className}`} draggable={false} />;
}
