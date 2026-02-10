<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    theme?: 'light';
  }

  let { theme, class: className, ...restProps }: Props = $props();
</script>

<div class={['spinner', theme, className]} {...restProps}>
  <span class="sr-only">Loading…</span>
</div>

<style>
  .spinner {
    --spinner-color: currentcolor;
    --spinner-bg-color: var(--gray-border);
    --spinner-size: 16px;

    display: inline-block;
    height: var(--spinner-size);
    width: var(--spinner-size);

    &:global(.light) {
      --spinner-bg-color: rgba(0, 0, 0, 0.2);
    }

    &::after {
      content: ' ';
      display: block;
      box-sizing: border-box;
      width: var(--spinner-size);
      height: var(--spinner-size);
      border-radius: 50%;
      border: calc(var(--spinner-size) / 5.5) solid var(--spinner-color);
      border-color: var(--spinner-bg-color) var(--spinner-bg-color) var(--spinner-color) var(--spinner-bg-color);
      animation: spinner 1.2s linear infinite;
    }
  }

  @keyframes spinner {
    0% {
      transform: rotate(0deg);
    }
    100% {
      transform: rotate(360deg);
    }
  }
</style>
