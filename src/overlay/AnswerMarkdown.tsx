import { useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

function MermaidBlock({ chart }: { chart: string }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    import("mermaid").then(async (mod) => {
      const mermaid = mod.default;
      mermaid.initialize({ startOnLoad: false, theme: "neutral", securityLevel: "strict" });
      const id = `diagram-${Math.random().toString(36).slice(2)}`;
      try {
        const rendered = await mermaid.render(id, chart.trim());
        if (!cancelled && ref.current) ref.current.innerHTML = rendered.svg;
      } catch {
        if (!cancelled && ref.current) ref.current.textContent = chart;
      }
    }).catch(() => {
      if (!cancelled && ref.current) ref.current.textContent = chart;
    });
    return () => {
      cancelled = true;
    };
  }, [chart]);

  return <div ref={ref} className="my-3 overflow-x-auto bg-background p-2" />;
}

export function AnswerMarkdown({ content }: { content: string }) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        code({ className, children }) {
          const text = String(children).replace(/\n$/, "");
          if (className?.includes("language-mermaid")) {
            return <MermaidBlock chart={text} />;
          }
          return <code className={className}>{children}</code>;
        },
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
