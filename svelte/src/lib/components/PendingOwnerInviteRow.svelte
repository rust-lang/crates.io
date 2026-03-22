<script lang="ts">
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';
  import { formatDistanceToNow } from 'date-fns';

  import { getNotifications } from '$lib/notifications.svelte';

  interface Invite {
    crate_id: number;
    crate_name: string;
    inviter_login: string;
    created_at: string;
  }

  interface Props {
    invite: Invite;
  }

  let { invite }: Props = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let result: 'pending' | 'accepted' | 'declined' = $state('pending');
  let isLoading = $state(false);

  async function handleInvitation(accepted: boolean) {
    let action = accepted ? 'accepting' : 'declining';

    isLoading = true;
    try {
      let response = await client.PUT('/api/v1/me/crate_owner_invitations/{crate_id}', {
        params: { path: { crate_id: invite.crate_id } },
        body: { crate_owner_invite: { crate_id: invite.crate_id, accepted } },
      });

      if (response.response.ok) {
        result = accepted ? 'accepted' : 'declined';
      } else {
        let detail = (response.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        if (detail && !detail.startsWith('{')) {
          notifications.error(`Error in ${action} invite: ${detail}`);
        } else {
          notifications.error(`Error in ${action} invite`);
        }
      }
    } catch {
      notifications.error(`Error in ${action} invite`);
    } finally {
      isLoading = false;
    }
  }
</script>

{#if result === 'accepted'}
  <p data-test-accepted-message data-test-invite={invite.crate_name}>
    Success! You've been added as an owner of crate
    <a href={resolve('/crates/[crate_id]', { crate_id: invite.crate_name })}>{invite.crate_name}</a>.
  </p>
{:else if result === 'declined'}
  <p data-test-declined-message data-test-invite={invite.crate_name}>
    Declined. You have not been added as an owner of crate
    <a href={resolve('/crates/[crate_id]', { crate_id: invite.crate_name })}>{invite.crate_name}</a>.
  </p>
{:else}
  <div class="row" data-test-invite={invite.crate_name}>
    <div class="crate-column">
      <h3>
        <a href={resolve('/crates/[crate_id]', { crate_id: invite.crate_name })} data-test-crate-link>
          {invite.crate_name}
        </a>
      </h3>
    </div>
    <div>
      Invited by:
      <a href={resolve('/users/[user_id]', { user_id: invite.inviter_login })} data-test-inviter-link>
        {invite.inviter_login}
      </a>
    </div>
    <div class="text--small" data-test-date>
      {formatDistanceToNow(invite.created_at, { addSuffix: true })}
    </div>
    <div>
      <button
        type="button"
        class="button button--small"
        data-test-accept-button
        disabled={isLoading}
        onclick={() => handleInvitation(true)}
      >
        Accept
      </button>
      <button
        type="button"
        class="button button--small"
        data-test-decline-button
        disabled={isLoading}
        onclick={() => handleInvitation(false)}
      >
        Decline
      </button>
    </div>
  </div>
{/if}

<style>
  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .crate-column {
    width: 200px;

    h3 {
      margin: 0;
    }
  }
</style>
