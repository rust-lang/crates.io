<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  let { data } = $props();

  let result: 'loading' | 'success' | 'error' = $state('loading');
  let errorText: string | undefined = $state();

  // Using `onMount` instead of `load()` because this is a mutation (PUT),
  // not data loading. `onMount` only runs in the browser, which avoids
  // issues with SSR and preloading re-firing the mutation.
  onMount(async () => {
    let client = createClient({ fetch });

    try {
      let response = await client.PUT('/api/v1/me/crate_owner_invitations/accept/{token}', {
        params: { path: { token: data.token } },
      });

      if (response.response.ok) {
        result = 'success';
      } else {
        errorText = (response.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        result = 'error';
      }
    } catch {
      result = 'error';
    }
  });
</script>

{#if result === 'success'}
  <h1>You've been added as a crate owner!</h1>
  <p data-test-success-message>
    Visit your
    <a href={resolve('/dashboard')}>dashboard</a>
    to view all of your crates, or
    <a href={resolve('/me')}>account settings</a>
    to manage email notification preferences for all of your crates.
  </p>
{:else if result === 'error'}
  <h1>Error in accepting crate ownership.</h1>
  <p data-test-error-message>
    {#if errorText}
      {errorText}
    {:else}
      You may want to visit
      <a href={resolve('/me/pending-invites')}>crates.io/me/pending-invites</a>
      to try again.
    {/if}
  </p>
{/if}
