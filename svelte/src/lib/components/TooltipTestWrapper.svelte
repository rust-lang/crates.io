<script lang="ts">
  import Tooltip from './Tooltip.svelte';

  interface Props {
    text: string;
    width: string;
    side?: 'top' | 'bottom' | 'left' | 'right';
    marginTop?: string;
    delay?: number;
    onlyWhenTruncated?: boolean;
    truncationTargetWidth?: string;
  }

  let {
    text,
    width,
    side,
    marginTop = '100px',
    delay = 0,
    onlyWhenTruncated = false,
    truncationTargetWidth,
  }: Props = $props();
  let truncationTarget = $state<HTMLSpanElement>();
</script>

<div
  data-test-anchor
  style:width
  style:margin-top={marginTop}
  style:overflow="hidden"
  style:white-space="nowrap"
  style:text-overflow="ellipsis"
>
  {#if truncationTargetWidth}
    <span
      bind:this={truncationTarget}
      style:display="inline-block"
      style:width={truncationTargetWidth}
      style:overflow="hidden"
      style:white-space="nowrap"
      style:text-overflow="ellipsis"
    >
      {text}
    </span>
  {:else}
    {text}
  {/if}
  <Tooltip {text} {side} {delay} {onlyWhenTruncated} {truncationTarget} />
</div>
