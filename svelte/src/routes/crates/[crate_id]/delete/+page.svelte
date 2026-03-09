<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import Alert from '$lib/components/Alert.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  let { data } = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let reason = $state('');
  let isConfirmed = $state(false);
  let isDeleting = $state(false);

  let crateName = $derived(data.crate.name);
  let canSubmit = $derived(isConfirmed && reason.length > 0 && !isDeleting);

  async function deleteCrate(event: SubmitEvent) {
    event.preventDefault();

    isDeleting = true;
    try {
      let result = await client.DELETE('/api/v1/crates/{name}', {
        params: {
          path: { name: crateName },
          query: { message: reason },
        },
      });

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? '');
      }

      notifications.success(`Crate ${crateName} has been successfully deleted.`);
      await goto(resolve('/'));
    } catch (error) {
      let message = 'Failed to delete crate';
      if (error instanceof Error && error.message) {
        message += `: ${error.message}`;
      }
      notifications.error(message);
    } finally {
      isDeleting = false;
    }
  }
</script>

<PageTitle title="Delete Crate" />

<div class="wrapper">
  <form class="content" onsubmit={deleteCrate}>
    <h1 class="title" data-test-title>Delete the {crateName} crate?</h1>

    <p>Are you sure you want to delete the crate "{crateName}"?</p>

    <Alert variant="warning">
      <strong>Important:</strong>
      This action will permanently delete the crate and its associated versions. Deleting a crate cannot be reversed!
    </Alert>

    <div class="impact">
      <h3>Potential Impact:</h3>
      <ul>
        <li>Users will no longer be able to download this crate.</li>
        <li>Any dependencies or projects relying on this crate will be broken.</li>
        <li>Deleted crates cannot be restored.</li>
        <li>Publishing a crate with the same name will be blocked for 24 hours.</li>
      </ul>
    </div>

    <div class="requirements">
      <h3>Requirements:</h3>
      <p>A crate can only be deleted if it is not depended upon by any other crate on crates.io.</p>
      <p>Additionally, a crate can only be deleted if either:</p>
      <ol class="first">
        <li>the crate has been published for less than 72 hours</li>
      </ol>
      <div class="or">or</div>
      <ol start={2} class="second">
        <li>
          <ol>
            <li>the crate only has a single owner, <em>and</em></li>
            <li>the crate has been downloaded less than 1000 times for each month it has been published.</li>
          </ol>
        </li>
      </ol>
    </div>

    <div class="reason">
      <h3>Reason:</h3>
      <label>
        <p>Please tell us why you are deleting this crate:</p>
        <input type="text" bind:value={reason} required class="reason-input base-input" data-test-reason />
      </label>
    </div>

    <Alert variant="warning" hideIcon>
      <label class="confirmation">
        <input type="checkbox" bind:checked={isConfirmed} disabled={isDeleting} data-test-confirmation-checkbox />
        I understand that deleting this crate is permanent and cannot be undone.
      </label>
    </Alert>

    <div class="actions">
      <button type="submit" disabled={!canSubmit} class="button button--red" data-test-delete-button>
        Delete this crate
      </button>
      {#if isDeleting}
        <div class="spinner-wrapper">
          <LoadingSpinner class="spinner" data-test-spinner />
        </div>
      {/if}
    </div>
  </form>
</div>

<style>
  .wrapper {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    place-items: center;
    margin: var(--space-s);
  }

  .content {
    max-width: 100%;
    overflow-wrap: break-word;
  }

  .title {
    margin-top: 0;
  }

  .impact,
  .requirements {
    li {
      margin-bottom: var(--space-2xs);
    }
  }

  @counter-style sub {
    system: extends lower-alpha;
    prefix: '(';
    suffix: ') ';
  }

  .requirements {
    .or {
      padding-left: 3.5em;
      padding-bottom: 0.3em;
      font-weight: bold;
      font-variant: small-caps;
    }

    .first {
      margin-bottom: 0.5em;
    }

    .second {
      margin-top: 0.5em;
    }

    :global(ol ol) {
      list-style-type: sub;
      padding-left: 1.5em;
    }
  }

  .reason {
    margin-bottom: var(--space-m);
  }

  .reason-input {
    width: 100%;
  }

  .confirmation {
    :global(input) {
      margin-right: var(--space-3xs);
    }
  }

  .actions {
    margin-top: var(--space-m);
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .spinner-wrapper {
    position: relative;
  }

  .spinner-wrapper > :global(.spinner) {
    position: absolute;
    --spinner-size: 1.5em;
    top: calc(-0.5 * var(--spinner-size));
    margin-left: var(--space-xs);
  }
</style>
