<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLUListElement> {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();
</script>

<ul role="list" class={['list', className]} {...restProps}>
  {@render children()}
</ul>

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3xs);
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>
