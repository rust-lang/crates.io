<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import ChevronRightIcon from '$lib/assets/chevron-right.svg?component';
  import Placeholder from '../Placeholder.svelte';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    withSubtitle?: boolean;
  }

  let { withSubtitle = false, class: className, ...restProps }: Props = $props();
</script>

<div class={['link', className]} {...restProps}>
  <div class="left">
    <Placeholder width="150px" height="16px" radius="8px" opacity={0.25} />
    {#if withSubtitle}
      <Placeholder
        class="subtitle"
        width="90px"
        height="13px"
        radius="6.5px"
        opacity={0.2}
        style="margin-top: var(--space-3xs)"
      />
    {/if}
  </div>
  <ChevronRightIcon class="right" />
</div>

<style>
  .link {
    --shadow: 0 2px 3px light-dark(hsla(51, 50%, 44%, 0.35), #232321);
    --placeholder-bg: light-dark(hsla(59, 19%, 50%, 1), hsl(60, 14%, 85%));
    --placeholder-bg2: light-dark(hsla(59, 19%, 50%, 0.7), hsla(59, 5%, 50%, 0.7));

    display: flex;
    align-items: center;
    width: 100%;
    height: var(--space-2xl);
    margin: 8px 0;
    padding: 0 var(--space-s);
    background-color: light-dark(white, #141413);
    color: light-dark(#525252, #f9f7ec);
    border-radius: var(--space-3xs);
    box-shadow: var(--shadow);
    cursor: wait;
  }

  .left {
    flex-grow: 1;
    width: 0;
  }

  .link :global(.right) {
    flex-shrink: 0;
    height: var(--space-s);
    width: auto;
    margin-left: var(--space-xs);
    color: light-dark(rgb(118, 131, 138), #cccac2);
  }
</style>
