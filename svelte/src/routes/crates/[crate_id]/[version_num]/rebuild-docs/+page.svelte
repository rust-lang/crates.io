<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import PageTitle from '$lib/components/PageTitle.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  let { data } = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let isRebuilding = $state(false);

  let crateName = $derived(data.crate.name);
  let versionNum = $derived(data.version.num);

  async function confirmRebuild() {
    isRebuilding = true;
    try {
      let result = await client.POST('/api/v1/crates/{name}/{version}/rebuild_docs', {
        params: { path: { name: crateName, version: versionNum } },
      });

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? 'Failed to enqueue docs rebuild task.');
      }

      notifications.success('Docs rebuild task was enqueued successfully!');
      await goto(resolve(`/crates/${crateName}/versions`));
    } catch (error) {
      let reason = error instanceof Error && error.message ? error.message : 'Failed to enqueue docs rebuild task.';
      notifications.error(`Error: ${reason}`);
    } finally {
      isRebuilding = false;
    }
  }
</script>

<PageTitle title="Rebuild Documentation" />

<div class="content">
  <h1 data-test-title>Rebuild Documentation</h1>

  <div class="crate-info">
    <h2>Crate Information</h2>
    <div class="info-row">
      <strong>Crate:</strong>
      <span data-test-crate-name>{crateName}</span>
    </div>
    <div class="info-row">
      <strong>Version:</strong>
      <span data-test-version-num>{versionNum}</span>
    </div>
  </div>

  <div class="description">
    <p>
      This will trigger a rebuild of the documentation for
      <a href="https://docs.rs/{crateName}/{versionNum}" target="_blank" rel="noopener noreferrer">
        <strong>{crateName} {versionNum}</strong>
      </a>
      on docs.rs.
    </p>
    <p>
      The rebuild process may take several minutes to complete. You can monitor the build progress at the
      <a href="https://docs.rs/releases/queue" target="_blank" rel="noopener noreferrer">docs.rs build queue</a>.
    </p>
  </div>

  <div class="actions">
    <button
      type="button"
      class="button button--yellow"
      disabled={isRebuilding}
      data-test-confirm-rebuild-button
      onclick={confirmRebuild}
    >
      {#if isRebuilding}
        Requesting Rebuild...
      {:else}
        Confirm Rebuild
      {/if}
    </button>
    <a href={resolve(`/crates/${crateName}/versions`)} class="button button--tan" data-test-cancel-button>Cancel</a>
  </div>
</div>

<style>
  .content {
    max-width: 600px;
    margin: var(--space-xl) auto;

    h1 {
      margin-top: 0;
    }
  }

  .crate-info {
    background-color: light-dark(var(--orange-50), var(--orange-900));
    border-radius: 8px;
    padding: var(--space-m);
    margin: var(--space-m) 0;
    border: 1px solid light-dark(var(--orange-200), var(--orange-600));

    h2 {
      margin: 0 0 var(--space-s);
    }
  }

  .info-row {
    margin-top: var(--space-xs);
  }

  .description {
    margin: var(--space-m) 0;
  }

  .actions {
    margin-top: var(--space-m);
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-s);
  }
</style>
