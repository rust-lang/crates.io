<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import UserAvatar from '$lib/components/UserAvatar.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  type Owner = components['schemas']['Owner'];

  let { data } = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let crate_id = $derived(data.crate.id);
  let crateName = $derived(data.crate.name);

  let owners = $derived([...data.owners]);
  let teamOwners = $derived(owners.filter(o => o.kind === 'team'));
  let userOwners = $derived(owners.filter(o => o.kind === 'user'));

  function ownerHref(owner: Owner): string {
    return owner.kind === 'team'
      ? resolve('/teams/[team_id]', { team_id: owner.login })
      : resolve('/users/[user_id]', { user_id: owner.login });
  }

  function teamDisplayName(owner: Owner): string {
    let orgName = owner.login.split(':')[1];
    return owner.name ? `${orgName}/${owner.name}` : owner.login;
  }

  async function removeOwner(owner: Owner) {
    let isTeam = owner.kind === 'team';
    let displayName = isTeam ? teamDisplayName(owner) : owner.login;
    let subject = isTeam ? `team ${displayName}` : `user ${displayName}`;

    try {
      let result = await client.DELETE('/api/v1/crates/{name}/owners', {
        params: { path: { name: crateName } },
        body: { owners: [owner.login] },
      });

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? '');
      }

      owners = owners.filter(o => !(o.kind === owner.kind && o.id === owner.id));

      if (isTeam) {
        notifications.success(`Team ${displayName} removed as crate owner`);
      } else {
        notifications.success(`User ${displayName} removed as crate owner`);
      }
    } catch (error) {
      let message = `Failed to remove the ${subject} as crate owner`;
      if (error instanceof Error && error.message) {
        message += `: ${error.message}`;
      }
      notifications.error(message);
    }
  }
</script>

<PageTitle title="Manage Crate Settings" />

<CrateHeader crate={data.crate} ownersPromise={data.ownersPromise} />

<div class="header">
  <h2>Owners</h2>
</div>

<div class="list" data-test-owners>
  {#each teamOwners as team (team.id)}
    {@const href = ownerHref(team)}
    <div class="row" data-test-owner-team={team.login}>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() is used above -->
      <a {href}>
        <UserAvatar user={team} size="medium-small" />
      </a>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a {href}>
        {teamDisplayName(team)}
      </a>
      <div class="email-column"></div>
      <button
        type="button"
        class="button button--small"
        data-test-remove-owner-button
        onclick={() => removeOwner(team)}
      >
        Remove
      </button>
    </div>
  {/each}
  {#each userOwners as user (user.id)}
    {@const href = ownerHref(user)}
    <div class="row" data-test-owner-user={user.login}>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a {href}>
        <UserAvatar {user} size="medium-small" />
      </a>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
      <a {href}>
        {user.name ?? user.login}
      </a>
      <div class="email-column"></div>
      <button
        type="button"
        class="button button--small"
        data-test-remove-owner-button
        onclick={() => removeOwner(user)}
      >
        Remove
      </button>
    </div>
  {/each}
</div>

<!-- TODO: Trusted Publishing section -->

<h2 class="header">Danger Zone</h2>
<div>
  <a href={resolve('/crates/[crate_id]/delete', { crate_id })} class="button button--red" data-test-delete-button>
    Delete this crate
  </a>
</div>

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-s);
    margin: var(--space-m) 0;
  }

  .header > h2 {
    margin: 0;
  }

  .list {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    > * {
      padding: var(--space-s) var(--space-m);
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
    }

    > * + * {
      border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
    }
  }

  .email-column {
    width: 25%;
    color: var(--main-color-light);
  }
</style>
