<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import Icon from '$lib/components/Icon.svelte';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    value: string;
    label: string;
    iconClass: string;
  }

  let { value, label, iconClass, class: className, ...restProps }: Props = $props();
</script>

<div class={['stats-value', className]} {...restProps}>
  <span class="value" data-test-value>{value}</span>
  <span class="label">{label}</span>
  <Icon class={iconClass} />
</div>

<style>
  .stats-value {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    justify-items: end;
    --icon-size: var(--space-xl-2xl);
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
    margin-left: var(--space-s);
    margin-top: calc(-1 * var(--space-4xs-3xs));
    color: light-dark(#545454, #adc0cd);
  }
</style>
