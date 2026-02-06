<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import OwnersList from './OwnersList.svelte';

  const { Story } = defineMeta({
    title: 'OwnersList',
    component: OwnersList,
  });

  const JANE_DOE = {
    id: 1,
    kind: 'user',
    name: 'Jane Doe',
    login: 'janedoe',
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
  };

  const JOHN_SMITH = {
    id: 2,
    kind: 'user',
    name: 'John Smith',
    login: 'johnsmith',
    avatar: 'https://avatars.githubusercontent.com/u/2?v=4',
  };

  const ANONYMOUS = {
    id: 3,
    kind: 'user',
    name: null,
    login: 'anonymous',
    avatar: 'https://avatars.githubusercontent.com/u/3?v=4',
  };

  const RUST_TEAM = {
    id: 5430905,
    kind: 'team',
    name: 'core',
    login: 'github:crates-io:core',
    avatar: 'https://avatars.githubusercontent.com/u/5430905?v=4',
  };

  const ANOTHER_TEAM = {
    id: 5430906,
    kind: 'team',
    name: 'admins',
    login: 'github:crates-io:admins',
    avatar: 'https://avatars.githubusercontent.com/t/5430906?v=4',
  };

  const USER_4 = {
    id: 4,
    kind: 'user',
    name: 'User Four',
    login: 'user4',
    avatar: 'https://avatars.githubusercontent.com/u/4?v=4',
  };

  const USER_5 = {
    id: 5,
    kind: 'user',
    name: 'User Five',
    login: 'user5',
    avatar: 'https://avatars.githubusercontent.com/u/5?v=4',
  };

  const USER_6 = {
    id: 6,
    kind: 'user',
    name: 'User Six',
    login: 'user6',
    avatar: 'https://avatars.githubusercontent.com/u/6?v=4',
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
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
