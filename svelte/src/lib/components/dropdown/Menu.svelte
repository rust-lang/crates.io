<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import Content from './Content.svelte';
  import { getDropdown } from './context';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();

  let dropdown = getDropdown();
</script>

<Content class={className} {...restProps}>
  <ul class="list" role="menu" aria-labelledby={dropdown.triggerId}>
    {@render children()}
  </ul>
</Content>

<style>
  .list {
    margin: 0;
    text-align: left;
    padding: 0;
    background: light-dark(white, #141413);
    border: 1px solid var(--gray-border);
    list-style: none;
    overflow: hidden;
    border-radius: var(--space-3xs);
  }
</style>
