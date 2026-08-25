<!--
  @component
  Renders the public user identity header.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
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

<PageHeader style="display: flex; align-items: center; gap: var(--space-xs);" data-test-heading>
  <UserAvatar
    user={{ avatar: user.avatar, kind: 'user', login: user.login, name: user.name }}
    size="medium"
    data-test-avatar
  />
  <h1 data-test-username>{user.login}</h1>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
  <a href={user.url} title={user.login} class="github-link" data-test-user-link>
    <Icon class="i-simple-icons:github" label="GitHub profile" />
  </a>
</PageHeader>

<style>
  h1 {
    margin: 0;
  }

  .github-link {
    --icon-size: 32px;

    &,
    &:hover {
      color: var(--main-color);
    }
  }
</style>
