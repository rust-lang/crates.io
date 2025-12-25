<script lang="ts">
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';

  interface Props {
    isLoading: boolean;
    onRetry?: () => void;
  }

  let { isLoading, onRetry }: Props = $props();
</script>

<p class="error-message" data-test-error-message>
  Unfortunately something went wrong while loading the crates.io summary data. Feel free to try again, or let the
  <a href="mailto:help@crates.io">crates.io team</a>
  know if the problem persists.
</p>

<button type="button" disabled={isLoading} class="try-again-button button" onclick={onRetry} data-test-try-again-button>
  Try Again
  {#if isLoading}
    <LoadingSpinner theme="light" class="spinner" data-test-spinner />
  {/if}
</button>

<style>
  .error-message {
    line-height: 1.5;
  }

  .try-again-button {
    align-self: center;
    margin: var(--space-s) 0;

    :global(.spinner) {
      margin-left: var(--space-2xs);
    }
  }
</style>
