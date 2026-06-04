<script lang="ts">
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
  class="follow-button button button--tan"
  {onclick}
>
  {#if status === 'loading'}
    <LoadingSpinner theme="light" />
  {:else if status === 'following'}
    Unfollow
  {:else}
    Follow
  {/if}
</button>

<style>
  .follow-button {
    height: 48px;
    width: 150px;
    justify-content: center;
  }
</style>
