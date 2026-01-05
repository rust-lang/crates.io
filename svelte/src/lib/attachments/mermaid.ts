import type { NotificationsContext } from '$lib/notifications.svelte';
import type { Mermaid } from 'mermaid';

let mermaidPromise: Promise<void> | null = null;
let mermaid: Mermaid | null = null;

export function loadMermaid(): Promise<void> {
  if (!mermaidPromise) {
    mermaidPromise = import('mermaid').then(m => {
      mermaid = m.default;
      mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });
    });
  }
  return mermaidPromise;
}

export function renderMermaids(html: string, notifications: NotificationsContext) {
  return (element: Element) => {
    void html;

    if (!mermaid) return;

    let nodes = element.querySelectorAll<HTMLElement>('.language-mermaid');
    if (nodes.length === 0) return;

    mermaid.run({ nodes }).catch(error => {
      console.error(error.error ?? error);
      notifications.warning('Failed to render mermaid diagram.');
    });
  };
}
