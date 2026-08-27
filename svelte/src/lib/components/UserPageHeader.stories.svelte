<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UserPageHeader from './UserPageHeader.svelte';

  type User = ComponentProps<typeof UserPageHeader>['user'];
  type LinkedAccount = ComponentProps<typeof UserPageHeader>['linkedAccounts'][number];

  const { Story } = defineMeta({
    title: 'UserPageHeader',
    component: UserPageHeader,
    tags: ['autodocs'],
  });

  const USER: User = {
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
    github_username_matches: true,
    login: 'janedoe',
    name: 'Jane Doe',
  };

  const LINKED_ACCOUNT: LinkedAccount = {
    account_id: '1',
    login: 'janedoe',
    provider: 'github',
  };

  const OTHER_LINKED_ACCOUNT: LinkedAccount = {
    account_id: '2',
    login: 'jane-doe',
    provider: 'github',
  };

  const USER_WITHOUT_NAME: User = {
    ...USER,
    name: null,
  };

  const USER_WITHOUT_AVATAR: User = {
    ...USER,
    avatar: null,
  };

  const USER_WITH_LONG_IDENTITY: User = {
    ...USER,
    login: 'averylongcratesiousernamethatcannotfitintheavailableheaderwidth',
    name: 'Alexandria Cassandra Montgomery-Worthington the Third',
  };

  const LONG_LINKED_ACCOUNT: LinkedAccount = {
    account_id: '3',
    login: USER_WITH_LONG_IDENTITY.login,
    provider: 'github',
  };

  const FOO_LINKED_ACCOUNT: LinkedAccount = {
    account_id: '4',
    login: 'foo',
    provider: 'github',
  };

  const BAR_LINKED_ACCOUNT: LinkedAccount = {
    account_id: '5',
    login: 'bar',
    provider: 'github',
  };
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Default</h1>
  <UserPageHeader user={USER} linkedAccounts={[LINKED_ACCOUNT]} />

  <h1>Without Name</h1>
  <UserPageHeader user={USER_WITHOUT_NAME} linkedAccounts={[LINKED_ACCOUNT]} />

  <h1>Without Avatar</h1>
  <UserPageHeader user={USER_WITHOUT_AVATAR} linkedAccounts={[LINKED_ACCOUNT]} />

  <h1>Multiple Accounts</h1>
  <UserPageHeader user={USER} linkedAccounts={[OTHER_LINKED_ACCOUNT, LINKED_ACCOUNT]} />

  <h1>Mismatched Account</h1>
  <UserPageHeader user={{ ...USER, github_username_matches: false }} linkedAccounts={[OTHER_LINKED_ACCOUNT]} />

  <h1>Without Accounts</h1>
  <UserPageHeader user={USER} linkedAccounts={[]} />

  <h1>Long Identity</h1>
  <div class="long-identity">
    <UserPageHeader
      user={USER_WITH_LONG_IDENTITY}
      linkedAccounts={[LONG_LINKED_ACCOUNT, FOO_LINKED_ACCOUNT, BAR_LINKED_ACCOUNT]}
    />
  </div>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }

  .long-identity {
    width: 500px;
    max-width: 100%;
  }
</style>
