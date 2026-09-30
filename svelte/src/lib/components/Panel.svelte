<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { SvelteHTMLElements } from 'svelte/elements';

  type Props = SvelteHTMLElements['div'] & {
    children: Snippet;
  };

  let { children, class: className, ...restProps }: Props = $props();
</script>

<div class={['panel', className]} {...restProps}>
  {@render children()}
</div>

<style>
  .panel {
    background-color: var(--surface);
    border-radius: var(--border-radius-small);
    box-shadow: 0 1px 3px var(--surface-shadow-color);
  }
</style>
