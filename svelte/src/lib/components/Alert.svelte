<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import AlertCautionIcon from '$lib/assets/alert-caution.svg?component';
  import AlertImportantIcon from '$lib/assets/alert-important.svg?component';
  import AlertNoteIcon from '$lib/assets/alert-note.svg?component';
  import AlertTipIcon from '$lib/assets/alert-tip.svg?component';
  import AlertWarningIcon from '$lib/assets/alert-warning.svg?component';
  import CheckCircleIcon from '$lib/assets/check-circle.svg?component';

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
      <AlertNoteIcon />
    {:else if variant === 'success'}
      <CheckCircleIcon />
    {:else if variant === 'tip'}
      <AlertTipIcon />
    {:else if variant === 'important'}
      <AlertImportantIcon />
    {:else if variant === 'warning'}
      <AlertWarningIcon />
    {:else if variant === 'caution'}
      <AlertCautionIcon />
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

  .alert :global(svg) {
    flex-shrink: 0;
    width: 1em;
    height: 1em;
    margin-right: var(--space-xs);
  }

  .alert[data-variant='note'] {
    background-color: light-dark(hsl(213, 93%, 90%), hsl(213, 50%, 20%));
    border-color: hsl(213, 93%, 62%);
  }

  .alert[data-variant='note'] :global(svg) {
    color: hsl(213, 93%, 62%);
  }

  .alert[data-variant='tip'],
  .alert[data-variant='success'] {
    background-color: light-dark(hsl(128, 49%, 90%), hsl(128, 30%, 20%));
    border-color: hsl(128, 49%, 49%);
  }

  .alert[data-variant='tip'] :global(svg),
  .alert[data-variant='success'] :global(svg) {
    color: hsl(128, 49%, 49%);
  }

  .alert[data-variant='important'] {
    background-color: light-dark(hsl(262, 90%, 90%), hsl(262, 50%, 20%));
    border-color: hsl(262, 90%, 73%);
  }

  .alert[data-variant='important'] :global(svg) {
    color: hsl(262, 90%, 73%);
  }

  .alert[data-variant='warning'] {
    background-color: light-dark(var(--yellow100), var(--yellow900));
    border-color: var(--yellow500);
  }

  .alert[data-variant='warning'] :global(svg) {
    color: var(--yellow500);
  }

  .alert[data-variant='caution'] {
    background-color: light-dark(hsl(3, 93%, 90%), hsl(3, 50%, 20%));
    border-color: hsl(3, 93%, 63%);
  }

  .alert[data-variant='caution'] :global(svg) {
    color: hsl(3, 93%, 63%);
  }

  .alert-content {
    text-wrap: pretty;
  }
</style>
