<script module lang="ts">
  import type { ComponentProps } from 'svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Avatar from './Avatar.svelte';

  type AvatarOwner = ComponentProps<typeof Avatar>['owner'];

  const { Story } = defineMeta({
    title: 'Avatar',
    component: Avatar,
  });

  const JANE_DOE: AvatarOwner = {
    kind: 'user',
    name: 'Jane Doe',
    login: 'janedoe',
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
  };

  const ANON_123: AvatarOwner = {
    kind: 'user',
    name: null,
    login: 'anon123',
    avatar: 'https://avatars.githubusercontent.com/u/2?v=4',
  };

  const RUST_TEAM: AvatarOwner = {
    kind: 'team',
    name: 'Rust Team',
    login: 'rust-lang',
    avatar: 'https://avatars.githubusercontent.com/u/5430905?v=4',
  };

  const EXAMPLES = [
    { label: 'User', owner: JANE_DOE },
    { label: 'Null Name', owner: ANON_123 },
    { label: 'Team', owner: RUST_TEAM },
  ];

  const SIZES = ['19.5px', '22px', '32px', '85px'];
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  {#each EXAMPLES as { label, owner } (label)}
    <h1>{label}</h1>
    <div class="row">
      {#each SIZES as size (size)}
        <Avatar {owner} style="--avatar-size: {size}" />
      {/each}
    </div>

    <h1>{label}, No Avatar</h1>
    <div class="row">
      {#each SIZES as size (size)}
        <Avatar owner={{ ...owner, avatar: null }} style="--avatar-size: {size}" />
      {/each}
    </div>
  {/each}

  <h1>Mismatched</h1>
  <div class="row">
    {#each SIZES as size (size)}
      <Avatar owner={JANE_DOE} mismatched style="--avatar-size: {size}" />
    {/each}
  </div>

  <h1>Header Shadow</h1>
  <div class="row header">
    {#each SIZES as size (size)}
      <Avatar owner={JANE_DOE} style="--avatar-size: {size}" />
    {/each}
  </div>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .header {
    --avatar-shadow: 1px 2px 2px rgba(0, 0, 0, 0.15);
    padding: 8px;
    background: var(--header-bg-color);
  }
</style>
