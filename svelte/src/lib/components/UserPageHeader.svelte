<!--
  @component
  Renders the public user identity header.
-->
<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import AccountChip from './AccountChip.svelte';
  import PageHeader from './PageHeader.svelte';
  import UserAvatar from './UserAvatar.svelte';

  type LinkedAccount = Pick<components['schemas']['LinkedAccount'], 'account_id' | 'login' | 'provider'>;

  interface UserPageHeaderUser {
    /** The user's avatar URL, if available. */
    avatar?: string | null;

    /** The username displayed in the page heading. */
    login: string;

    /** Whether a linked GitHub username exactly matches the crates.io username. */
    github_username_matches: boolean;

    /** The user's optional display name. */
    name?: string | null;
  }

  interface Props {
    /** The public user identity displayed in the header. */
    user: UserPageHeaderUser;

    /** The external accounts linked to the user. */
    linkedAccounts: LinkedAccount[];
  }

  let { user, linkedAccounts }: Props = $props();

  function buildUrl(account: LinkedAccount): string {
    switch (account.provider) {
      case 'github':
        return `https://github.com/${account.login}`;
    }
  }
</script>

<PageHeader data-test-heading>
  <div class="layout">
    <UserAvatar
      user={{ avatar: user.avatar, kind: 'user', login: user.login, name: user.name }}
      size="medium"
      class="user-page-avatar"
      data-test-avatar
    />
    <div class="identity">
      <h1 data-test-username>{user.login}</h1>
      {#if user.name}
        <div class="display-name" data-test-display-name>{user.name}</div>
      {/if}
      {#if linkedAccounts.length !== 0}
        <div class="accounts">
          {#each linkedAccounts as account (account.account_id)}
            <AccountChip
              provider={account.provider}
              handle={account.login}
              href={buildUrl(account)}
              mismatched={account.provider === 'github' && !user.github_username_matches}
            />
          {/each}
        </div>
      {/if}
    </div>
  </div>
</PageHeader>

<style>
  .layout {
    display: flex;
    align-items: center;
    gap: var(--space-s);
  }

  .identity {
    flex: 1;
    min-width: 0;
  }

  h1 {
    margin: 0;
    line-height: 1.1;
    overflow-wrap: anywhere;
  }

  .display-name {
    margin-top: var(--space-3xs);
    color: var(--main-color-light);
    font-size: 0.9375em;
    overflow-wrap: anywhere;
  }

  .accounts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2xs);
    margin-top: var(--space-2xs);
  }

  :global(.user-page-avatar) {
    align-self: start;
    flex-shrink: 0;
    border-radius: 50%;
    object-fit: cover;
    background: white;
    padding: 3px;
    box-shadow: 1px 2px 2px 0 light-dark(hsla(51, 50%, 44%, 0.35), #232321);
  }
</style>
