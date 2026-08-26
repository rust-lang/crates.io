<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UserPageHeader from './UserPageHeader.svelte';

  type User = ComponentProps<typeof UserPageHeader>['user'];

  const { Story } = defineMeta({
    title: 'UserPageHeader',
    component: UserPageHeader,
    tags: ['autodocs'],
  });

  const USER: User = {
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
    login: 'janedoe',
    name: 'Jane Doe',
    url: 'https://github.com/janedoe',
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
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Default</h1>
  <UserPageHeader user={USER} />

  <h1>Without Name</h1>
  <UserPageHeader user={USER_WITHOUT_NAME} />

  <h1>Without Avatar</h1>
  <UserPageHeader user={USER_WITHOUT_AVATAR} />

  <h1>Long Identity</h1>
  <div class="long-identity">
    <UserPageHeader user={USER_WITH_LONG_IDENTITY} />
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
