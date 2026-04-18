import { type ForwardedRef, forwardRef, useMemo } from "react";
import { EDITOR_CONSTANTS } from "@/features/editor/config/constants";
import { type SemanticToken, TOKEN_TYPE_NAMES } from "./use-semantic-tokens";

interface SemanticTokensOverlayProps {
  tokens: SemanticToken[];
  content: string;
  fontSize: number;
  charWidth: number;
  scrollTop: number;
  viewportHeight: number;
}

// Map token type indices to CSS color classes
const TOKEN_TYPE_COLORS: Record<string, string> = {
  namespace: "text-cyan-400/80",
  type: "text-teal-400/80",
  class: "text-yellow-400/80",
  enum: "text-orange-300/80",
  interface: "text-cyan-300/80",
  struct: "text-yellow-300/80",
  typeParameter: "text-teal-300/80",
  parameter: "text-orange-200/70",
  variable: "text-blue-300/70",
  property: "text-green-300/70",
  enumMember: "text-orange-300/70",
  function: "text-purple-300/80",
  method: "text-purple-300/80",
  macro: "text-rose-300/80",
  decorator: "text-yellow-200/80",
};

const SemanticTokensOverlay = forwardRef(
  (
    { tokens, content, fontSize, charWidth, scrollTop, viewportHeight }: SemanticTokensOverlayProps,
    ref: ForwardedRef<HTMLDivElement>,
  ) => {
    const lineHeight = Math.ceil(fontSize * EDITOR_CONSTANTS.LINE_HEIGHT_MULTIPLIER);

    const visibleTokens = useMemo(() => {
      const buffer = viewportHeight * 0.5;
      const startLine = Math.floor(Math.max(0, scrollTop - buffer) / lineHeight);
      const endLine = Math.ceil((scrollTop + viewportHeight + buffer) / lineHeight) + 1;

      return tokens.filter((t) => t.line >= startLine && t.line <= endLine);
    }, [tokens, scrollTop, viewportHeight, lineHeight]);

    const lines = useMemo(() => content.split("\n"), [content]);

    if (visibleTokens.length === 0) return null;

    return (
      <div
        ref={ref}
        className="pointer-events-none absolute inset-0 overflow-hidden"
        style={{ zIndex: 3 }}
      >
        {visibleTokens.map((token) => {
          const typeName = TOKEN_TYPE_NAMES[token.tokenType];
          const colorClass = typeName ? TOKEN_TYPE_COLORS[typeName] : undefined;

          if (!colorClass) return null;

          const top = EDITOR_CONSTANTS.EDITOR_PADDING_TOP + token.line * lineHeight;
          const left = EDITOR_CONSTANTS.EDITOR_PADDING_LEFT + token.startChar * charWidth;
          const width = token.length * charWidth;

          const lineContent = lines[token.line] || "";
          const tokenText = lineContent.slice(token.startChar, token.startChar + token.length);

          return (
            <span
              key={`${token.line}:${token.startChar}:${token.length}`}
              className={`absolute editor-font ${colorClass}`}
              style={{
                top: `${top}px`,
                left: `${left}px`,
                width: `${width}px`,
                fontSize: `${fontSize}px`,
                lineHeight: `${lineHeight}px`,
              }}
            >
              {tokenText}
            </span>
          );
        })}
      </div>
    );
  },
);

SemanticTokensOverlay.displayName = "SemanticTokensOverlay";

export default SemanticTokensOverlay;
