<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import Icon from '$lib/components/Icon.svelte';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    variant: 'note' | 'success' | 'tip' | 'important' | 'warning' | 'caution';
    hideIcon?: boolean;
    children: Snippet;
  }

  let { variant, hideIcon = false, children, class: className, ...others }: Props = $props();
</script>

<div data-test-alert class={['alert', className]} data-variant={variant} {...others}>
  {#if !hideIcon}
    {#if variant === 'note'}
      <Icon class="i-octicon:info-16" />
    {:else if variant === 'success'}
      <Icon class="i-octicon:check-circle-16" />
    {:else if variant === 'tip'}
      <Icon class="i-octicon:light-bulb-16" />
    {:else if variant === 'important'}
      <Icon class="i-octicon:report-16" />
    {:else if variant === 'warning'}
      <Icon class="i-octicon:alert-16" />
    {:else if variant === 'caution'}
      <Icon class="i-octicon:stop-16" />
    {/if}
  {/if}
  <div class="alert-content">
    {@render children()}
  </div>
</div>

<style>
  .alert {
    display: flex;
    padding: var(--space-xs);
    border-left-style: solid;
    border-left-width: 4px;
    border-radius: var(--space-3xs);
  }

  .alert :global(.icon) {
    margin-right: var(--space-xs);
  }

  .alert[data-variant='note'] {
    background-color: light-dark(hsl(213, 93%, 90%), hsl(213, 50%, 20%));
    border-color: hsl(213, 93%, 62%);
  }

  .alert[data-variant='note'] :global(.icon) {
    color: hsl(213, 93%, 62%);
  }

  .alert[data-variant='tip'],
  .alert[data-variant='success'] {
    background-color: light-dark(hsl(128, 49%, 90%), hsl(128, 30%, 20%));
    border-color: hsl(128, 49%, 49%);
  }

  .alert[data-variant='tip'] :global(.icon),
  .alert[data-variant='success'] :global(.icon) {
    color: hsl(128, 49%, 49%);
  }

  .alert[data-variant='important'] {
    background-color: light-dark(hsl(262, 90%, 90%), hsl(262, 50%, 20%));
    border-color: hsl(262, 90%, 73%);
  }

  .alert[data-variant='important'] :global(.icon) {
    color: hsl(262, 90%, 73%);
  }

  .alert[data-variant='warning'] {
    background-color: light-dark(var(--yellow100), var(--yellow900));
    border-color: var(--yellow500);
  }

  .alert[data-variant='warning'] :global(.icon) {
    color: var(--yellow500);
  }

  .alert[data-variant='caution'] {
    background-color: light-dark(hsl(3, 93%, 90%), hsl(3, 50%, 20%));
    border-color: hsl(3, 93%, 63%);
  }

  .alert[data-variant='caution'] :global(.icon) {
    color: hsl(3, 93%, 63%);
  }

  .alert-content {
    text-wrap: pretty;
  }
</style>
