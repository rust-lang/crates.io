<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UserAvatar from './UserAvatar.svelte';

  const { Story } = defineMeta({
    title: 'UserAvatar',
    component: UserAvatar,
    argTypes: {
      size: {
        control: 'select',
        options: ['small', 'medium-small', 'medium'],
      },
    },
  });

  const JANE_DOE = {
    id: 1,
    kind: 'user',
    name: 'Jane Doe',
    login: 'janedoe',
    avatar: 'https://avatars.githubusercontent.com/u/1?v=4',
  };

  const ANON_123 = {
    id: 2,
    kind: 'user',
    name: null,
    login: 'anon123',
    avatar: 'https://avatars.githubusercontent.com/u/2?v=4',
  };

  const RUST_TEAM = {
    id: 5430905,
    kind: 'team',
    name: 'Rust Team',
    login: 'rust-lang',
    avatar: 'https://avatars.githubusercontent.com/u/5430905?v=4',
  };
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Small (default)</h1>
  <UserAvatar user={JANE_DOE} size="small" />

  <h1>Medium Small</h1>
  <UserAvatar user={JANE_DOE} size="medium-small" />

  <h1>Medium</h1>
  <UserAvatar user={JANE_DOE} size="medium" />

  <h1>Null Name</h1>
  <UserAvatar user={ANON_123} size="medium" />

  <h1>Team</h1>
  <UserAvatar user={RUST_TEAM} size="medium" />

  <h1>All Sizes</h1>
  <div style="display: flex; align-items: center; gap: 16px;">
    <UserAvatar user={JANE_DOE} size="small" />
    <UserAvatar user={JANE_DOE} size="medium-small" />
    <UserAvatar user={JANE_DOE} size="medium" />
  </div>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
