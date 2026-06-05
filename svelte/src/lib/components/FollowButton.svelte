<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';

  interface Props {
    /** Which follow state the button represents. */
    status: 'loading' | 'following' | 'not-following';
    /** Disables the button independently of the loading state. */
    disabled?: boolean;
    /** Called when the user clicks the button. */
    onclick: () => void;
  }

  let { status, disabled = false, onclick }: Props = $props();
</script>

<button
  type="button"
  disabled={disabled || status === 'loading'}
  data-test-follow-button
  class="follow-button button-reset"
  {onclick}
>
  {#if status === 'loading'}
    <LoadingSpinner theme="light" style="--spinner-size: 0.8em" label={null} /> Loading
  {:else if status === 'following'}
    <Icon class="i-mdi:bell" /> Unfollow
  {:else}
    <Icon class="i-mdi:bell-outline" /> Follow
  {/if}
</button>

<style>
  .follow-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-3xs);
    border: 1px solid var(--gray-border);
    border-radius: 99999px;
    font-size: var(--space-xs);
    font-weight: 500;
    padding: var(--space-3xs) var(--space-xs);
    cursor: pointer;

    transition:
      border-color var(--transition-fast),
      background var(--transition-fast);

    &:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }

    &:hover:not(:disabled),
    &:focus:not(:disabled) {
      border-color: var(--yellow500);
      background: light-dark(#fffdf5, #1b1b18);
    }
  }
</style>
