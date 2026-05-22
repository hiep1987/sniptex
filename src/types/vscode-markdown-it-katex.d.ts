// `@vscode/markdown-it-katex` ships no .d.ts. Expose only the
// plugin entry point we use; the runtime accepts a KatexOptions-shaped
// options object, but we pass just throwOnError + errorColor.

declare module "@vscode/markdown-it-katex" {
  import type MarkdownIt from "markdown-it";
  type KatexOptions = {
    throwOnError?: boolean;
    errorColor?: string;
    macros?: Record<string, string>;
    displayMode?: boolean;
    enableMathInlineInHtml?: boolean;
    enableBareBlocks?: boolean;
    enableFencedBlocks?: boolean;
  };
  type Plugin = (md: MarkdownIt, options?: KatexOptions) => void;
  const plugin: Plugin;
  export default plugin;
}
