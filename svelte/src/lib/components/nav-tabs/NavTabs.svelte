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
    --nav-tabs-radius: var(--space-3xs);

    display: flex;
    list-style: none;
    padding: 0;
    margin: 0;

    @media only screen and (min-width: 551px) {
      border-bottom: var(--nav-tabs-border-width) solid var(--gray-border);
    }

    @media only screen and (max-width: 550px) {
      flex-direction: column;
      border-left: var(--nav-tabs-border-width) solid var(--gray-border);
    }
  }
</style>
