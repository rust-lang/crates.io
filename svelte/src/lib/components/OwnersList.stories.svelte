<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import OwnersList from './OwnersList.svelte';

  type Owner = components['schemas']['Owner'];

  const { Story } = defineMeta({
    title: 'OwnersList',
    component: OwnersList,
  });

  const JANE_DOE: Owner = {
    id: 1,
    kind: 'user',
    name: 'Jane Doe',
    login: 'janedoe',
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
    url: 'https://github.com/janedoe',
    github_username_matches: true,
  };

  const JOHN_SMITH: Owner = {
    id: 2,
    kind: 'user',
    name: 'John Smith',
    login: 'johnsmith',
    avatar: 'https://avatars.githubusercontent.com/u/2?v=4',
    url: 'https://github.com/johnsmith',
    github_username_matches: true,
  };

  const ANONYMOUS: Owner = {
    id: 3,
    kind: 'user',
    name: null,
    login: 'anonymous',
    avatar: 'https://avatars.githubusercontent.com/u/3?v=4',
    url: 'https://github.com/anonymous',
    github_username_matches: true,
  };

  const RUST_TEAM: Owner = {
    id: 5_430_905,
    kind: 'team',
    name: 'core',
    login: 'github:crates-io:core',
    avatar: 'https://avatars.githubusercontent.com/u/5430905?v=4',
    url: 'https://github.com/crates-io',
  };

  const ANOTHER_TEAM: Owner = {
    id: 5_430_906,
    kind: 'team',
    name: 'admins',
    login: 'github:crates-io:admins',
    avatar: 'https://avatars.githubusercontent.com/t/5430906?v=4',
    url: 'https://github.com/crates-io',
  };

  const USER_4: Owner = {
    id: 4,
    kind: 'user',
    name: 'User Four',
    login: 'user4',
    avatar: 'https://avatars.githubusercontent.com/u/4?v=4',
    url: 'https://github.com/user4',
    github_username_matches: true,
  };

  const USER_5: Owner = {
    id: 5,
    kind: 'user',
    name: 'User Five',
    login: 'user5',
    avatar: 'https://avatars.githubusercontent.com/u/5?v=4',
    url: 'https://github.com/user5',
    github_username_matches: true,
  };

  const USER_6: Owner = {
    id: 6,
    kind: 'user',
    name: 'User Six',
    login: 'user6',
    avatar: 'https://avatars.githubusercontent.com/u/6?v=4',
    url: 'https://github.com/user6',
    github_username_matches: true,
  };

  const NO_AVATAR_USER: Owner = {
    id: 7,
    kind: 'user',
    name: 'Avatarless Andy',
    login: 'avatarless',
    avatar: null,
    url: 'https://github.com/avatarless',
    github_username_matches: true,
  };

  const NO_AVATAR_ANON: Owner = {
    id: 8,
    kind: 'user',
    name: null,
    login: 'noname',
    avatar: null,
    url: 'https://github.com/noname',
    github_username_matches: true,
  };

  const NO_AVATAR_TEAM: Owner = {
    id: 9,
    kind: 'team',
    name: 'maintainers',
    login: 'github:crates-io:maintainers',
    avatar: null,
    url: 'https://github.com/crates-io',
  };
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Single User</h1>
  <OwnersList owners={[JANE_DOE]} />

  <h1>User Without Name</h1>
  <OwnersList owners={[ANONYMOUS]} />

  <h1>Five Users (Detailed)</h1>
  <OwnersList owners={[JANE_DOE, JOHN_SMITH, ANONYMOUS, USER_4, USER_5]} />

  <h1>Six Users (Compact)</h1>
  <OwnersList owners={[JANE_DOE, JOHN_SMITH, ANONYMOUS, USER_4, USER_5, USER_6]} />

  <h1>Mixed Users and Teams</h1>
  <OwnersList owners={[RUST_TEAM, ANOTHER_TEAM, JANE_DOE, JOHN_SMITH, ANONYMOUS]} />

  <h1>Teams Only</h1>
  <OwnersList owners={[RUST_TEAM, ANOTHER_TEAM]} />

  <h1>Without Avatars (Detailed)</h1>
  <OwnersList owners={[NO_AVATAR_USER, NO_AVATAR_ANON, NO_AVATAR_TEAM]} />

  <h1>Without Avatars (Compact)</h1>
  <OwnersList owners={[NO_AVATAR_USER, NO_AVATAR_ANON, NO_AVATAR_TEAM, JANE_DOE, JOHN_SMITH, USER_4]} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
