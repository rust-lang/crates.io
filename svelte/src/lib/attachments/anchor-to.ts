import type { Placement } from '@floating-ui/dom';
import type { Attachment } from 'svelte/attachments';

import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom';

export interface AnchorOptions {
  placement: Placement;
  /** Distance between the anchor and the element in pixels. */
  offset: number;
  /** Minimum distance between the element and the viewport edges in pixels. */
  padding: number;
}

/**
 * Svelte attachment that positions an absolutely positioned element next to
 * `anchor`, flipping and shifting it to stay inside the viewport.
 *
 * The position updates whenever the anchor or the element moves or resizes.
 */
export function anchorTo(anchor: HTMLElement, options: AnchorOptions): Attachment<HTMLElement> {
  return element => {
    async function updatePosition() {
      let { x, y } = await computePosition(anchor, element, {
        placement: options.placement,
        middleware: [offset(options.offset), flip(), shift({ padding: options.padding })],
      });

      Object.assign(element.style, {
        left: `${x}px`,
        top: `${y}px`,
      });
    }

    return autoUpdate(anchor, element, updatePosition);
  };
}
