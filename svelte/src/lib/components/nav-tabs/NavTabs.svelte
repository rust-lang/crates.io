<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();
</script>

<nav class={className} {...restProps}>
  <ul class="list">
    {@render children()}
  </ul>
</nav>

<style>
  .list {
    --nav-tabs-border-width: 2px;
    --nav-tabs-padding-h: var(--space-s);
    --nav-tabs-padding-v: var(--space-2xs);
    --nav-tabs-radius: var(--border-radius-small);

    display: flex;
    list-style: none;
    padding: 0;
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
</style>
