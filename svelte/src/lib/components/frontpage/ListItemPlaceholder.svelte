<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import Icon from '$lib/components/Icon.svelte';
  import Placeholder from '../Placeholder.svelte';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    withSubtitle?: boolean;
    withTrailing?: boolean;
  }

  let { withSubtitle = false, withTrailing = false, class: className, ...restProps }: Props = $props();
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
  {#if withTrailing}
    <Placeholder width="40px" height="13px" radius="6.5px" opacity={0.2} />
  {:else}
    <Icon class="i-mdi:chevron-right right" />
  {/if}
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
    height: var(--space-m);
    width: var(--space-m);
    margin-right: calc(-0.8 * var(--space-2xs));
    color: light-dark(rgb(118, 131, 138), #cccac2);
  }
</style>
