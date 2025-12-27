<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLSpanElement> {
    start: number;
    end: number;
    total: number;
    name?: string;
  }

  let { start, end, total, name, ...restProps }: Props = $props();
</script>

<span class="text--small" {...restProps}>
  Displaying
  <span class="highlight" data-test-current-rows>{start}-{end}</span>
  of
  <span class="highlight" data-test-total-rows>{total}</span>
  {name ?? 'total results'}
</span>

<style>
  .highlight {
    color: var(--main-color);
    font-weight: bold;
  }
</style>
