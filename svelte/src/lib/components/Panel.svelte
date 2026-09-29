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
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
  }
</style>
