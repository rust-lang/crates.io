import type { Attachment } from 'svelte/attachments';

/**
 * Svelte attachment that calls a callback when user clicks outside the element.
 *
 * Uses capture phase to ensure we catch the event before any onclick handlers
 * that might remove elements from the DOM.
 */
export function clickOutside(callback: () => void): Attachment {
  return element => {
    function handleClick(event: MouseEvent) {
      let target = event.target as Node;

      // Check if click target still exists in DOM (handles edge case where
      // clicked element was removed before this handler fires)
      if (!document.body.contains(target)) {
        return;
      }

      // Check if click is outside the node
      if (!element.contains(target)) {
        callback();
      }
    }

    // Use capture phase (third argument = true) to fire before element removal
    document.addEventListener('click', handleClick, true);

    return () => {
      document.removeEventListener('click', handleClick, true);
    };
  };
}
