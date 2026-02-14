import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { Transformer } from "markmap-lib";
import { Markmap } from "markmap-view";

const transformer = new Transformer();

export interface MarkmapViewerHandle {
  getSvgHtml: () => string | null;
}

export const MarkmapViewer = forwardRef<
  MarkmapViewerHandle,
  { markdown: string }
>(function MarkmapViewer({ markdown }, ref) {
  const svgRef = useRef<SVGSVGElement>(null);
  const mmRef = useRef<Markmap | null>(null);
  const markdownRef = useRef(markdown);
  markdownRef.current = markdown;

  useImperativeHandle(ref, () => ({
    getSvgHtml: () => svgRef.current?.outerHTML ?? null,
  }), []);

  // Mount markmap instance
  useEffect(() => {
    if (!svgRef.current) return;
    const mm = Markmap.create(svgRef.current, {
      autoFit: true,
      duration: 300,
      maxWidth: 250,
      spacingHorizontal: 80,
      spacingVertical: 8,
    });
    mmRef.current = mm;

    // Set data if available
    if (markdownRef.current) {
      const { root } = transformer.transform(markdownRef.current);
      void mm.setData(root).then(() => mm.fit());
    }
    return () => {
      mm.destroy();
      mmRef.current = null;
    };
  }, []);

  // Update data on markdown change
  useEffect(() => {
    if (!mmRef.current || !markdown) return;
    const { root } = transformer.transform(markdown);
    const mm = mmRef.current;
    void mm.setData(root).then(() => mm.fit());
  }, [markdown]);

  return (
    <svg ref={svgRef} className="h-full w-full" style={{ minHeight: "200px" }} />
  );
});
