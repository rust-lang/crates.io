<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Avatar from './Avatar.svelte';
  import UsernameMismatchNote from './UsernameMismatchNote.svelte';

  const { Story } = defineMeta({
    title: 'UsernameMismatchNote',
    component: UsernameMismatchNote,
    tags: ['autodocs'],
  });

  const BOB_SMITH = {
    kind: 'user' as const,
    name: 'Bob Smith',
    login: 'bobsmith',
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
  };
</script>

<Story name="Default" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
  <a href="/users/{BOB_SMITH.login}" class="owner">
    <Avatar owner={BOB_SMITH} mismatched aria-hidden="true" style="--avatar-size: 32px" />
    {BOB_SMITH.name}<UsernameMismatchNote login={BOB_SMITH.login} />
  </a>
</Story>

<style>
  .owner {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
</style>
