<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { Attachment } from 'svelte/attachments';

  import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom';

  import { getTooltipContext } from '$lib/tooltip.svelte';

  interface BaseProps {
    side?: 'top' | 'bottom' | 'left' | 'right';
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

  let { text, children, side = 'top' }: Props = $props();

  let anchorElement = $state<HTMLElement | null>(null);
  let visible = $state(false);
  let hideTimeout = $state<ReturnType<typeof setTimeout> | null>(null);

  function show() {
    if (hideTimeout) {
      clearTimeout(hideTimeout);
      hideTimeout = null;
    }
    visible = true;
  }

  function hide() {
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
    container?.appendChild(floatingElement);

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
