<!--
  @component
  Native `popover` anchored to a button, which toggles it. The browser closes
  it on Escape and when clicking outside of it, renders it in the top layer,
  and places it after the anchor in the Tab order. It is rendered at the end
  of `<body>` so it doesn't inherit styles from its parent.
-->
<script lang="ts">
  import type { Placement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';
  import type { Attachment } from 'svelte/attachments';
  import type { HTMLAttributes, ToggleEventHandler } from 'svelte/elements';

  import { anchorTo } from '#lib/attachments/anchor-to.ts';
  import Portal from './portal/Portal.svelte';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    open: boolean;
    anchor: HTMLButtonElement | undefined;
    placement?: Placement;
    onBeforeToggle?: ToggleEventHandler<HTMLDivElement>;
    onToggle?: ToggleEventHandler<HTMLDivElement>;
    children: Snippet;
  }

  let {
    open = $bindable(),
    anchor,
    placement = 'bottom-start',
    children,
    class: className,
    onBeforeToggle,
    onToggle,
    ...restProps
  }: Props = $props();

  // The content is rendered as soon as the popover starts to show, and removed
  // only after it has hidden, so the browser can return focus out of it.
  let rendered = $state(false);

  let linkAnchor: Attachment<HTMLDivElement> = element => {
    let button = anchor;
    if (!button) return;

    button.popoverTargetElement = element;
    return () => (button.popoverTargetElement = null);
  };

  let syncOpen: Attachment<HTMLDivElement> = element => {
    if (open === element.matches(':popover-open')) return;

    if (open) {
      element.showPopover({ source: anchor });
    } else {
      element.hidePopover();
    }
  };

  // An anchor hidden while the popover is open, for example by a media query,
  // would leave the popover open at the top left of the viewport.
  let closeWhenAnchorHidden: Attachment<HTMLDivElement> = element => {
    let button = anchor;
    if (!button || !rendered) return;

    let observer = new ResizeObserver(() => {
      if (button.checkVisibility?.() === false) element.hidePopover();
    });
    observer.observe(button);
    return () => observer.disconnect();
  };

  function handleBeforeToggle(event: ToggleEvent & { currentTarget: HTMLDivElement }) {
    onBeforeToggle?.(event);

    // Only opening can be cancelled, and no `toggle` event follows then.
    if (event.defaultPrevented) {
      open = false;
    } else if (event.newState === 'open') {
      rendered = true;
    }
  }

  function handleToggle(event: ToggleEvent & { currentTarget: HTMLDivElement }) {
    open = rendered = event.newState === 'open';
    onToggle?.(event);
  }
</script>

<Portal>
  <div
    {...restProps}
    popover="auto"
    class={['popover', className]}
    onbeforetoggle={handleBeforeToggle}
    ontoggle={handleToggle}
    {@attach linkAnchor}
    {@attach syncOpen}
    {@attach closeWhenAnchorHidden}
    {@attach anchor && rendered && anchorTo(anchor, { placement, offset: 0, padding: 5, strategy: 'fixed' })}
  >
    {#if rendered}
      {@render children()}
    {/if}
  </div>
</Portal>

<style>
  .popover {
    inset: 0 auto auto 0;
    margin: 0;
    padding: 0;
    border: none;
    overflow: visible;
    background: none;
    color: inherit;
  }
</style>
