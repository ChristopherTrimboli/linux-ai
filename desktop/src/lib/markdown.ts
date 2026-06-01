import { marked } from "marked";
import hljs from "highlight.js";
import DOMPurify from "dompurify";

marked.setOptions({ gfm: true, breaks: true });

export function renderMarkdown(src: string): string {
  const raw = marked.parse(src ?? "", { async: false }) as string;
  return DOMPurify.sanitize(raw);
}

/** Svelte action: render markdown into the node and syntax-highlight code. */
export function markdown(node: HTMLElement, text: string) {
  const update = (value: string) => {
    node.innerHTML = renderMarkdown(value);
    node.querySelectorAll<HTMLElement>("pre code").forEach((el) => {
      try {
        hljs.highlightElement(el);
      } catch {
        /* ignore highlight failures */
      }
    });
  };
  update(text);
  return {
    update,
  };
}
