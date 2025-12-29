<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLLiAttributes } from 'svelte/elements';

  interface Props extends HTMLLiAttributes {
    children: Snippet;
  }

  let { children, class: className, ...restProps }: Props = $props();
</script>

<li class={['item', className]} role="menuitem" {...restProps}>
  {@render children()}
</li>

<style>
  .item {
    > :global(a),
    > :global(button) {
      font-size: 90%;
      width: 100%;
      display: inline-flex;
      text-align: start;
      padding: var(--space-2xs) var(--space-xs);
      text-decoration: none;
      color: var(--main-color) !important;

      &:hover {
        background: #5e5e5e;
        color: white !important;
      }
    }
  }
</style>
