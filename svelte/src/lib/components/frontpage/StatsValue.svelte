<script lang="ts">
  import type { Component } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    value: string;
    label: string;
    icon: Component;
  }

  let { value, label, icon: Icon, class: className, ...restProps }: Props = $props();
</script>

<div class={['stats-value', className]} {...restProps}>
  <span class="value" data-test-value>{value}</span>
  <span class="label">{label}</span>
  <Icon role="img" aria-hidden="true" class="icon" />
</div>

<style>
  .stats-value {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    justify-items: end;
  }

  .value {
    font-size: var(--space-m-l);
    font-weight: bold;
    line-height: 1em;
  }

  .label {
    grid-column: 1;
    grid-row: 2;
    color: light-dark(#545454, #adc0cd);
    font-size: var(--space-s);
    font-weight: normal;
    line-height: 1.6;
  }

  .stats-value > :global(.icon) {
    grid-column: 2;
    grid-row: 1 / 3;
    width: var(--space-l-xl);
    height: var(--space-l-xl);
    margin-left: var(--space-s);
    margin-top: var(--space-3xs);
    color: light-dark(#545454, #adc0cd);
  }
</style>
