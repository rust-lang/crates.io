<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Attachment } from 'svelte/attachments';

  import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom';

  import { getTooltipContext } from '$lib/tooltip.svelte';

  interface BaseProps {
    side?: 'top' | 'bottom' | 'left' | 'right';
    /**
     * Milliseconds to wait after the pointer enters the anchor before showing
     * the tooltip. Defaults to `0`, which shows it immediately.
     */
    delay?: number;
    /**
     * Only show the tooltip when the anchor element's content is horizontally
     * truncated (e.g. clipped by `text-overflow: ellipsis`). Useful to avoid a
     * redundant tooltip that merely repeats fully visible text.
     */
    onlyWhenTruncated?: boolean;
  }

  interface TextProps extends BaseProps {
    text: string;
    children?: never;
  }

  interface ChildrenProps extends BaseProps {
    text?: never;
    children: Snippet;
  }

  type Props = TextProps | ChildrenProps;

  let { text, children, side = 'top', delay = 0, onlyWhenTruncated = false }: Props = $props();

  let anchorElement = $state<HTMLElement | null>(null);
  let visible = $state(false);
  let showTimeout = $state<ReturnType<typeof setTimeout> | null>(null);
  let hideTimeout = $state<ReturnType<typeof setTimeout> | null>(null);

  function show() {
    // Sub-pixel rounding can leave `scrollWidth` 1px above `clientWidth`
    // without any visible clipping, so require more than that before treating
    // the content as truncated.
    if (onlyWhenTruncated && anchorElement && anchorElement.scrollWidth - anchorElement.clientWidth <= 1) {
      return;
    }

    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }

    if (visible || showTimeout) return;

    showTimeout = setTimeout(() => {
      showTimeout = null;
      visible = true;
    }, delay);
  }

  function hide() {
    if (showTimeout) {
      clearTimeout(showTimeout);
      showTimeout = null;
    }

    hideTimeout = setTimeout(() => {
      visible = false;
    }, 100);
  }

  let attachAnchor: Attachment = element => {
    anchorElement = element.parentElement as HTMLElement | null;

    let events: [string, () => void][] = [
      ['mouseenter', show],
      ['mouseleave', hide],
      ['focus', show],
      ['blur', hide],
    ];

    for (let [event, listener] of events) {
      anchorElement?.addEventListener(event, listener);
    }

    return () => {
      for (let [event, listener] of events) {
        anchorElement?.removeEventListener(event, listener);
      }
    };
  };

  let attachTooltip: Attachment = element => {
    let floatingElement = element as HTMLElement;

    if (!anchorElement) return;

    let tooltipContext = getTooltipContext();
    let container = document.getElementById(tooltipContext.containerId);
    container?.append(floatingElement);

    async function updatePosition() {
      if (!anchorElement) return;

      let middleware = [offset(5), flip(), shift({ padding: 5 })];

      let { x, y } = await computePosition(anchorElement, floatingElement, {
        placement: side,
        middleware,
      });

      Object.assign(floatingElement.style, {
        left: `${x}px`,
        top: `${y}px`,
      });
    }

    let cleanupAutoUpdate = autoUpdate(anchorElement, floatingElement, updatePosition);

    floatingElement.addEventListener('mouseenter', show);
    floatingElement.addEventListener('mouseleave', hide);

    return () => {
      cleanupAutoUpdate();
      floatingElement.removeEventListener('mouseenter', show);
      floatingElement.removeEventListener('mouseleave', hide);
      floatingElement.remove();
    };
  };
</script>

<span class="anchor" {@attach attachAnchor}></span>

{#if visible}
  <div class="tooltip" {@attach attachTooltip}>
    {#if children}
      {@render children()}
    {:else}
      {text}
    {/if}
  </div>
{/if}

<style>
  .anchor {
    display: none;
  }

  .tooltip {
    width: max-content;
    max-width: 300px;
    position: absolute;
    top: 0;
    left: 0;
    background: #3a3c47;
    color: white;
    font-family: var(--font-body);
    font-size: 14px;
    font-weight: normal;
    overflow: hidden;
    text-wrap: auto;
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--space-3xs);
    z-index: 2;

    :global(strong) {
      color: unset;
    }
  }
</style>
