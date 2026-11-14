<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import { SvelteDate } from 'svelte/reactivity';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    daysOptions?: number[];
    date?: SvelteDate;
    disabled?: boolean;
    invalid?: boolean;
    noun: string;
  }

  let id = $props.id();
  let {
    daysOptions = [7, 30, 60, 90, 365],
    date = $bindable(),
    disabled = false,
    invalid = $bindable(false),
    noun,
    class: className,
    ...others
  }: Props = $props();

  let today = $derived(new Date());

  let selection: string | number = $state('none');
  // svelte-ignore state_referenced_locally
  let dateInput = $state(today.toISOString().slice(0, 10));

  let description = $derived(
    selection === 'none'
      ? `The ${noun} will never expire`
      : `The ${noun} will expire on ${date?.toLocaleDateString(undefined, { dateStyle: 'long' })}`,
  );

  function updateDate() {
    if (selection === 'none') {
      date = undefined;
    } else if (selection === 'custom') {
      date = new SvelteDate(dateInput);

      if (date < today) {
        invalid = true;
      }
    } else {
      date = new SvelteDate(Date.now() + Number(selection) * 24 * 60 * 60 * 1000);
    }
  }

  function updateSelection(event: Event) {
    dateInput = date?.toISOString().slice(0, 10) ?? '';
    selection = (event.target as HTMLSelectElement).value;

    updateDate();
  }
</script>

<div class={className} {...others}>
  <select {id} {disabled} class="expiry-select base-input" onchange={updateSelection} data-test-expiry>
    <option value="none" selected={selection === 'none'}>No expiration</option>
    {#each daysOptions as days (days)}
      <option value={days} selected={selection === days}>{days} day{days === 1 ? '' : 's'}</option>
    {/each}
    <option value="custom" selected={selection === 'custom'}>Custom...</option>
  </select>

  {#if selection === 'custom'}
    <input
      type="date"
      bind:value={dateInput}
      min={today.toISOString().slice(0, 10)}
      {disabled}
      aria-invalid={invalid}
      aria-label="Custom expiration date"
      class="expiry-date-input base-input"
      data-test-expiry-date
      oninput={() => {
        invalid = false;
        updateDate();
      }}
    />
  {:else}
    <span class="expiry-description" data-test-expiry-description>
      {description}
    </span>
  {/if}
</div>

<style>
  .expiry-select {
    --dropdown-icon-light: icon('i-mdi:menu-down', 'black');
    --dropdown-icon-dark: icon('i-mdi:menu-down', 'white');

    padding-right: var(--space-m);
    background-image: var(--dropdown-icon-light);
    background-repeat: no-repeat;
    background-position: calc(100% - var(--space-3xs)) center;
    background-size: 20px;
    appearance: none;

    :global([data-color-scheme='system']) & {
      @media (prefers-color-scheme: dark) {
        background-image: var(--dropdown-icon-dark);
      }
    }

    :global([data-color-scheme='dark']) & {
      background-image: var(--dropdown-icon-dark);
    }
  }

  .expiry-date-input {
    margin-left: var(--space-2xs);
  }

  .expiry-description {
    margin-left: var(--space-2xs);
    font-size: 0.9em;
  }
</style>
