<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import Icon from '#lib/components/Icon.svelte';

  interface Props extends HTMLAttributes<HTMLElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();

  let overflowStart = $state(false);
  let overflowEnd = $state(false);

  function trackOverflow(list: HTMLElement) {
    function update() {
      overflowStart = list.scrollLeft > 1;
      overflowEnd = list.scrollLeft + list.clientWidth < list.scrollWidth - 1;
    }

    // Tabs can be added after the first render (e.g. the crate settings tab),
    // which changes the scroll width without resizing the list itself.
    let resizeObserver = new ResizeObserver(update);
    resizeObserver.observe(list);
    let mutationObserver = new MutationObserver(update);
    mutationObserver.observe(list, { childList: true });
    list.addEventListener('scroll', update, { passive: true });

    return () => {
      resizeObserver.disconnect();
      mutationObserver.disconnect();
      list.removeEventListener('scroll', update);
    };
  }
</script>

<nav class={['nav', className]} {...restProps}>
  <ul class="list" {@attach trackOverflow}>
    {@render children()}
  </ul>
  <span class="hint start" class:visible={overflowStart}>
    <Icon class="i-mdi:chevron-left" />
  </span>
  <span class="hint end" class:visible={overflowEnd}>
    <Icon class="i-mdi:chevron-right" />
  </span>
</nav>

<style>
  .nav {
    --nav-tabs-border-width: 2px;
    --nav-tabs-padding-h: var(--space-s);
    --nav-tabs-padding-v: var(--space-2xs);
    --nav-tabs-radius: var(--border-radius-small);

    position: relative;
  }

  .list {
    display: flex;
    list-style: none;
    /* Lets callers extend the bar beyond the tabs while keeping them aligned */
    padding: 0 var(--nav-tabs-inset, 0);
    scroll-padding-inline: var(--nav-tabs-inset, 0);
    margin: 0;
    overflow-x: auto;
    overscroll-behavior-x: contain;
    scrollbar-width: none;
    /* An inset shadow instead of a border, so the active tab underline can
       cover it without overflowing the scroll container. */
    box-shadow: inset 0 calc(0px - var(--nav-tabs-border-width)) var(--gray-border);

    @media only screen and (max-width: 550px) {
      --nav-tabs-padding-h: var(--space-xs);
      font-size: 0.9em;
    }
  }

  /* Decorative only, so taps reach the tab underneath */
  .hint {
    position: absolute;
    top: 0;
    bottom: var(--nav-tabs-border-width);
    width: 40px;
    display: flex;
    align-items: center;
    padding-inline: var(--space-3xs);
    color: var(--main-color-light);
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--transition-fast);

    &.visible {
      opacity: 1;
    }
  }

  .start {
    left: 0;
    background: linear-gradient(to right, var(--main-bg) 40%, transparent);
  }

  .end {
    right: 0;
    justify-content: flex-end;
    background: linear-gradient(to left, var(--main-bg) 40%, transparent);
  }
</style>
