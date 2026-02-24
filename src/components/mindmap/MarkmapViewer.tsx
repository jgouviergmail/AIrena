import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { Transformer } from "markmap-lib";
import { Markmap } from "markmap-view";

const transformer = new Transformer();

export interface MarkmapViewerHandle {
  getSvgHtml: () => string | null;
}

/** Padding (px) added around the full content bounding box in exported SVGs. */
const SVG_EXPORT_PADDING = 20;

/**
 * Build a standalone SVG string that contains the **complete** argument map,
 * regardless of the current zoom/pan state in the application.
 *
 * Markmap renders inside a `<g>` with a D3 zoom transform — the live viewBox
 * only reflects the visible viewport.  To export the full tree we:
 *   1. Compute the bounding box of ALL content via `getBBox()` on the main `<g>`
 *      (returns coordinates in the `<g>`'s local system, before zoom transform).
 *   2. Set the clone's viewBox to that full bbox (with padding).
 *   3. Remove the zoom transform on the cloned `<g>` so content coordinates
 *      align directly with the viewBox.
 */
function buildStandaloneSvg(svg: SVGSVGElement): string | null {
  // The main <g> (first direct child group) holds all markmap nodes/links
  // and carries the D3 zoom transform.
  const mainG = svg.querySelector<SVGGElement>(":scope > g");
  if (!mainG) return null;

  // getBBox() returns the bounding box in <g>'s LOCAL coordinate system
  // (i.e. the natural layout, independent of zoom/pan).
  const bbox = mainG.getBBox();

  const clone = svg.cloneNode(true) as SVGSVGElement;

  // Ensure XML namespace attributes for standalone rendering
  clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  clone.setAttribute("xmlns:xlink", "http://www.w3.org/1999/xlink");

  // Set viewBox to encompass ALL content (not just the visible viewport)
  const vbX = Math.floor(bbox.x - SVG_EXPORT_PADDING);
  const vbY = Math.floor(bbox.y - SVG_EXPORT_PADDING);
  const vbW = Math.ceil(bbox.width + 2 * SVG_EXPORT_PADDING);
  const vbH = Math.ceil(bbox.height + 2 * SVG_EXPORT_PADDING);
  clone.setAttribute("viewBox", `${vbX} ${vbY} ${vbW} ${vbH}`);
  clone.setAttribute("width", String(vbW));
  clone.setAttribute("height", String(vbH));

  // Remove the D3 zoom/pan transform so content coordinates match the viewBox
  const cloneG = clone.querySelector<SVGGElement>(":scope > g");
  if (cloneG) {
    cloneG.removeAttribute("transform");
  }

  // Keep only markmap classes (selectors like .markmap, .mm-xxx) — strip Tailwind/app classes
  const existingClasses = (clone.getAttribute("class") || "").split(" ");
  const markmapClasses = existingClasses.filter(
    (c) => c && (c.startsWith("markmap") || c.startsWith("mm-")),
  );
  if (markmapClasses.length > 0) {
    clone.setAttribute("class", markmapClasses.join(" "));
  } else {
    clone.removeAttribute("class");
  }
  // Preserve markmap's --markmap-max-width CSS variable (controls text wrapping
  // inside foreignObject nodes), but strip Tailwind inline styles (minHeight etc.)
  const maxWidth = svg.style.getPropertyValue("--markmap-max-width");
  clone.removeAttribute("style");
  if (maxWidth) {
    clone.style.setProperty("--markmap-max-width", maxWidth);
  }

  // Serialize with proper XML declaration
  const serializer = new XMLSerializer();
  const svgString = serializer.serializeToString(clone);
  return `<?xml version="1.0" encoding="UTF-8"?>\n${svgString}`;
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
    getSvgHtml: () => svgRef.current ? buildStandaloneSvg(svgRef.current) : null,
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
