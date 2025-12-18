<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    width: string;
    height: string;
    radius?: string;
    opacity?: number;
  }

  let { width, height, radius = 'var(--space-3xs)', opacity = 0.35, class: className, ...restProps }: Props = $props();
</script>

<div
  class={['placeholder', className]}
  style:width
  style:height
  style:border-radius={radius}
  style:opacity
  {...restProps}
></div>

<style>
  .placeholder {
    position: relative;
    display: block;
    overflow: hidden;
    background: linear-gradient(
      to right,
      var(--placeholder-bg) 8%,
      var(--placeholder-bg2) 16%,
      var(--placeholder-bg) 29%
    );
    background-size: 1200px 100%;
    animation-name: backgroundAnimation;
    animation-duration: 1.5s;
    animation-timing-function: linear;
    animation-iteration-count: infinite;
    animation-fill-mode: forwards;
  }

  @keyframes backgroundAnimation {
    0% {
      background-position: -500px;
    }

    100% {
      background-position: 500px;
    }
  }
</style>
