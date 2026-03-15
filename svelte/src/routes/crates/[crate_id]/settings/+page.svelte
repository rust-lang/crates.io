<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { invalidateAll } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import Alert from '$lib/components/Alert.svelte';
  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import UserAvatar from '$lib/components/UserAvatar.svelte';
  import { getNotifications } from '$lib/notifications.svelte';

  type Owner = components['schemas']['Owner'];
  type GitHubConfig = components['schemas']['GitHubConfig'];
  type GitLabConfig = components['schemas']['GitLabConfig'];

  let { data } = $props();

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let addOwnerVisible = $state(false);
  let username = $state('');

  let crate_id = $derived(data.crate.id);
  let crateName = $derived(data.crate.name);

  let owners = $derived([...data.owners]);
  let teamOwners = $derived(owners.filter(o => o.kind === 'team'));
  let userOwners = $derived(owners.filter(o => o.kind === 'user'));

  let githubConfigs = $derived([...data.githubConfigs]);
  let gitlabConfigs = $derived([...data.gitlabConfigs]);
  let hasConfigs = $derived(githubConfigs.length > 0 || gitlabConfigs.length > 0);

  let trustpubOnlyOverride = $state<boolean | undefined>(undefined);
  let trustpubOnly = $derived(trustpubOnlyOverride ?? data.crate.trustpub_only);
  let trustpubOnlyLoading = $state(false);

  // Captured once on page load to prevent the checkbox from disappearing
  // when unchecked within the same page visit.
  // svelte-ignore state_referenced_locally
  let trustpubOnlyCheckboxWasVisible =
    data.githubConfigs.length > 0 || data.gitlabConfigs.length > 0 || data.crate.trustpub_only;
  let showTrustpubOnlyCheckbox = $derived(hasConfigs || trustpubOnly || trustpubOnlyCheckboxWasVisible);
  let showTrustpubOnlyWarning = $derived(trustpubOnly && !hasConfigs);

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

  async function addOwner(event: SubmitEvent) {
    event.preventDefault();

    let name = username;

    try {
      let result = await client.PUT('/api/v1/crates/{name}/owners', {
        params: { path: { name: crateName } },
        body: { owners: [name] },
      });

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? '');
      }

      if (name.includes(':')) {
        notifications.success(`Team ${name} was added as a crate owner`);
        await invalidateAll();
      } else {
        notifications.success(`An invite has been sent to ${name}`);
      }
      addOwnerVisible = false;
    } catch (error) {
      let message = 'Error sending invite';
      if (error instanceof Error && error.message) {
        message += `: ${error.message}`;
      }
      notifications.error(message);
    }
  }

  async function removeConfig(config: GitHubConfig | GitLabConfig, type: 'github' | 'gitlab') {
    try {
      let path =
        type === 'github'
          ? ('/api/v1/trusted_publishing/github_configs/{id}' as const)
          : ('/api/v1/trusted_publishing/gitlab_configs/{id}' as const);

      let result = await client.DELETE(path, {
        params: { path: { id: config.id } },
      });

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? '');
      }

      if (type === 'github') {
        githubConfigs = githubConfigs.filter(c => c.id !== config.id);
      } else {
        gitlabConfigs = gitlabConfigs.filter(c => c.id !== config.id);
      }

      notifications.success('Trusted Publishing configuration removed successfully');
    } catch (error) {
      let message = 'Failed to remove Trusted Publishing configuration';
      if (error instanceof Error && error.message) {
        message += `: ${error.message}`;
      }
      notifications.error(message);
    }
  }

  async function toggleTrustpubOnly(event: Event) {
    let { checked } = event.target as HTMLInputElement;
    trustpubOnlyLoading = true;
    try {
      let result = await client.PATCH('/api/v1/crates/{name}', {
        params: { path: { name: crateName } },
        body: { crate: { trustpub_only: checked } },
      });

      if (!result.response.ok) {
        let detail = (result.error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
        throw new Error(detail ?? '');
      }

      trustpubOnlyOverride = checked;
    } catch (error) {
      let message: string;
      if (error instanceof Error && error.message) {
        message = error.message;
      } else {
        message = 'Failed to update trusted publishing setting';
      }
      notifications.error(message);
    } finally {
      trustpubOnlyLoading = false;
    }
  }
</script>

<PageTitle title="Manage Crate Settings" />

<CrateHeader crate={data.crate} ownersPromise={data.ownersPromise} />

<div class="header">
  <h2>Owners</h2>
  {#if !addOwnerVisible}
    <button
      type="button"
      class="button button--small"
      data-test-add-owner-button
      onclick={() => {
        addOwnerVisible = true;
        username = '';
      }}
    >
      Add Owner
    </button>
  {/if}
</div>

{#if addOwnerVisible}
  <form class="add-owner-form" onsubmit={addOwner}>
    <label class="add-owner-label" for="new-owner-username">Username</label>
    <input
      type="text"
      id="new-owner-username"
      bind:value={username}
      placeholder="Username"
      class="add-owner-input"
      name="username"
    />
    <button type="submit" disabled={!username} class="button button--small" data-test-save-button>Add</button>
  </form>
{/if}

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

<div class="header">
  <h2>Trusted Publishing</h2>
  <div>
    <a
      href={resolve('/docs/trusted-publishing')}
      class="button button--tan button--small"
      data-test-trusted-publishing-docs-button
    >
      Learn more
    </a>
    <a
      href={resolve('/crates/[crate_id]/settings/new-trusted-publisher', { crate_id })}
      class="button button--small"
      data-test-add-trusted-publisher-button
    >
      Add
    </a>
  </div>
</div>

{#if showTrustpubOnlyWarning}
  <div class="trustpub-only-warning">
    <Alert variant="warning" data-test-trustpub-only-warning>
      Trusted publishing is required but no publishers are configured. Publishing to this crate is currently blocked.
    </Alert>
  </div>
{/if}

<div class="trustpub">
  <table data-test-trusted-publishing>
    <thead>
      <tr>
        <th>Publisher</th>
        <th>Details</th>
        <th><span class="sr-only">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each githubConfigs as config (config.id)}
        <tr data-test-github-config={config.id}>
          <td>GitHub</td>
          <td class="details">
            <strong>Repository:</strong>
            <a
              href="https://github.com/{config.repository_owner}/{config.repository_name}"
              target="_blank"
              rel="noopener noreferrer">{config.repository_owner}/{config.repository_name}</a
            >
            <span class="owner-id">
              · Owner ID: {config.repository_owner_id}
              <Tooltip>
                This is the owner ID for
                <strong>{config.repository_owner}</strong>
                from when this configuration was created. If
                <strong>{config.repository_owner}</strong>
                was recreated on GitHub, this configuration will need to be recreated as well.
              </Tooltip>
            </span>
            <br />
            <strong>Workflow:</strong>
            <a
              href="https://github.com/{config.repository_owner}/{config.repository_name}/blob/HEAD/.github/workflows/{config.workflow_filename}"
              target="_blank"
              rel="noopener noreferrer"
            >
              {config.workflow_filename}
            </a>
            <br />
            {#if config.environment}
              <strong>Environment:</strong> {config.environment}
            {/if}
          </td>
          <td class="actions">
            <button
              type="button"
              class="button button--small"
              data-test-remove-config-button
              onclick={() => removeConfig(config, 'github')}
            >
              Remove
            </button>
          </td>
        </tr>
      {/each}

      {#each gitlabConfigs as config (config.id)}
        <tr data-test-gitlab-config={config.id}>
          <td>GitLab</td>
          <td class="details">
            <strong>Repository:</strong>
            <a href="https://gitlab.com/{config.namespace}/{config.project}" target="_blank" rel="noopener noreferrer">
              {config.namespace}/{config.project}
            </a>
            <span class="owner-id">
              · Namespace ID:
              {#if config.namespace_id}
                {config.namespace_id}
                <Tooltip>
                  This is the namespace ID for
                  <strong>{config.namespace}</strong>
                  from the first publish using this configuration. If
                  <strong>{config.namespace}</strong>
                  was recreated on GitLab, this configuration will need to be recreated as well.
                </Tooltip>
              {:else}
                (not yet set)
                <Tooltip>The namespace ID will be captured from the first publish using this configuration.</Tooltip>
              {/if}
            </span><br />
            <strong>Workflow:</strong>
            <a
              href="https://gitlab.com/{config.namespace}/{config.project}/-/blob/HEAD/{config.workflow_filepath}"
              target="_blank"
              rel="noopener noreferrer"
            >
              {config.workflow_filepath}
            </a>
            <br />
            {#if config.environment}
              <strong>Environment:</strong> {config.environment}
            {/if}
          </td>
          <td class="actions">
            <button
              type="button"
              class="button button--small"
              data-test-remove-config-button
              onclick={() => removeConfig(config, 'gitlab')}
            >
              Remove
            </button>
          </td>
        </tr>
      {/each}

      {#if !hasConfigs}
        <tr class="no-trustpub-config" data-test-no-config>
          <td colspan="3">No trusted publishers configured for this crate.</td>
        </tr>
      {/if}
    </tbody>
  </table>

  {#if showTrustpubOnlyCheckbox}
    <label class="trustpub-only-checkbox" data-test-trustpub-only-checkbox>
      <div class="checkbox">
        {#if trustpubOnlyLoading}
          <LoadingSpinner data-test-spinner />
        {:else}
          <input type="checkbox" checked={trustpubOnly} data-test-checkbox onchange={toggleTrustpubOnly} />
        {/if}
      </div>
      <div class="label">Require trusted publishing for all new versions</div>
      <div class="note">
        When enabled, new versions can only be published through configured trusted publishers. Publishing with API
        tokens will be rejected.
      </div>
    </label>
  {/if}
</div>

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

  .add-owner-form {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-s);
    padding: var(--space-s) var(--space-m);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
    margin-bottom: var(--space-s);
  }

  .add-owner-label {
    font-weight: bold;
  }

  .add-owner-input {
    width: 400px;
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

  .trustpub {
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);
  }

  .trustpub table {
    width: 100%;
    border-spacing: 0;

    :global(tbody) > :global(tr) > :global(td) {
      border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
    }

    :global(th),
    :global(td) {
      text-align: left;
      padding: var(--space-s) var(--space-m);
    }

    .details {
      font-size: 0.85em;
      line-height: 1.5;

      .owner-id {
        color: var(--main-color-light);
      }
    }

    .actions {
      text-align: right;
    }

    @media only screen and (max-width: 550px) {
      thead {
        display: none;
      }

      tbody > tr:not(.no-trustpub-config) > td:first-child {
        padding-bottom: 0;
      }

      tbody > tr:not(:first-child) > td:first-child {
        border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
      }

      tbody > tr > td {
        border: none;
      }

      td {
        display: block;
        width: 100%;
      }

      .details {
        padding-bottom: 0;
      }

      .actions {
        text-align: left;
      }
    }
  }

  .trustpub-only-warning {
    margin-bottom: var(--space-s);
  }

  .trustpub-only-checkbox {
    display: grid;
    grid-template:
      'checkbox label' auto
      'checkbox note' auto / 16px 1fr;
    row-gap: var(--space-3xs);
    column-gap: var(--space-xs);
    padding: var(--space-s) var(--space-m);
    cursor: pointer;
    border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #232321);
  }

  .checkbox {
    grid-area: checkbox;
  }

  .label {
    grid-area: label;
    font-weight: bold;
  }

  .note {
    grid-area: note;
    font-size: 85%;
    color: var(--main-color-light);
  }
</style>
