/** Lightweight markdown: **bold**, bullet lists (- item), paragraphs (\n\n) */
export function SimpleMd({ text }: { text: string }) {
  const paragraphs = text.split(/\n{2,}/);
  return (
    <div className="space-y-3 text-sm text-foreground">
      {paragraphs.map((para, i) => {
        const lines = para.split("\n");
        const isList = lines.every((l) => /^[-*]\s/.test(l.trim()));
        if (isList) {
          return (
            <ul key={i} className="list-disc space-y-1 pl-5">
              {lines.map((l, j) => (
                <li key={j}><BoldText text={l.replace(/^[-*]\s+/, "")} /></li>
              ))}
            </ul>
          );
        }
        return <p key={i} className="leading-relaxed"><BoldText text={para} /></p>;
      })}
    </div>
  );
}

function BoldText({ text }: { text: string }) {
  const parts = text.split(/(\*\*[^*]+\*\*)/g);
  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith("**") && part.endsWith("**")) {
          return <strong key={i}>{part.slice(2, -2)}</strong>;
        }
        return <span key={i}>{part}</span>;
      })}
    </>
  );
}
