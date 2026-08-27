<!--
  @component
  Renders the public user identity header.
-->
<script lang="ts">
  import AccountChip from './AccountChip.svelte';
  import PageHeader from './PageHeader.svelte';
  import UserAvatar from './UserAvatar.svelte';

  interface UserPageHeaderUser {
    /** The user's avatar URL, if available. */
    avatar?: string | null;

    /** The username displayed in the page heading. */
    login: string;

    /** The user's optional display name. */
    name?: string | null;

    /** The URL of the user's GitHub profile. */
    url: string;
  }

  interface Props {
    /** The public user identity displayed in the header. */
    user: UserPageHeaderUser;
  }

  let { user }: Props = $props();
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
      <div class="accounts">
        <AccountChip provider="github" handle={user.login} href={user.url} />
      </div>
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
