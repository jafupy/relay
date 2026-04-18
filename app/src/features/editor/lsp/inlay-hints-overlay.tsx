import { type ForwardedRef, forwardRef, useMemo } from "react";
import { EDITOR_CONSTANTS } from "@/features/editor/config/constants";
import type { InlayHint } from "./use-inlay-hints";

interface InlayHintsOverlayProps {
  hints: InlayHint[];
  fontSize: number;
  charWidth: number;
  scrollTop: number;
  viewportHeight: number;
}

const InlayHintsOverlay = forwardRef(
  (
    { hints, fontSize, charWidth, scrollTop, viewportHeight }: InlayHintsOverlayProps,
    ref: ForwardedRef<HTMLDivElement>,
  ) => {
    const lineHeight = Math.ceil(fontSize * EDITOR_CONSTANTS.LINE_HEIGHT_MULTIPLIER);

    // Only render hints visible in the viewport (with buffer)
    const visibleHints = useMemo(() => {
      const buffer = viewportHeight * 0.5;
      const startLine = Math.floor(Math.max(0, scrollTop - buffer) / lineHeight);
      const endLine = Math.ceil((scrollTop + viewportHeight + buffer) / lineHeight) + 1;
      return hints.filter((h) => h.line >= startLine && h.line <= endLine);
    }, [hints, scrollTop, viewportHeight, lineHeight]);

    if (visibleHints.length === 0) return null;

    return (
      <div
        ref={ref}
        className="pointer-events-none absolute inset-0 overflow-hidden"
        style={{ zIndex: 5 }}
      >
        {visibleHints.map((hint) => {
          const top = EDITOR_CONSTANTS.EDITOR_PADDING_TOP + hint.line * lineHeight;
          const left = EDITOR_CONSTANTS.EDITOR_PADDING_LEFT + hint.character * charWidth;

          return (
            <span
              key={`${hint.line}:${hint.character}:${hint.label}`}
              className="absolute inline-flex items-center rounded-sm bg-hover/50 editor-font text-text-lighter/70"
              style={{
                top: `${top}px`,
                left: `${left}px`,
                fontSize: `${fontSize * 0.85}px`,
                lineHeight: `${lineHeight}px`,
                paddingLeft: hint.paddingLeft ? "3px" : "1px",
                paddingRight: hint.paddingRight ? "3px" : "1px",
              }}
            >
              {hint.label}
            </span>
          );
        })}
      </div>
    );
  },
);

InlayHintsOverlay.displayName = "InlayHintsOverlay";

export default InlayHintsOverlay;
