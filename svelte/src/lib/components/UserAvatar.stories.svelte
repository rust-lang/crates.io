<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UserAvatar from './UserAvatar.svelte';

  const { Story } = defineMeta({
    title: 'UserAvatar',
    component: UserAvatar,
    tags: ['autodocs'],
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

<Story name="Small (default)" args={{ user: JANE_DOE, size: 'small' }} />

<Story name="Medium Small" args={{ user: JANE_DOE, size: 'medium-small' }} />

<Story name="Medium" args={{ user: JANE_DOE, size: 'medium' }} />

<Story name="Null Name" args={{ user: ANON_123, size: 'medium' }} />

<Story name="Team" args={{ user: RUST_TEAM, size: 'medium' }} />

<Story name="All Sizes" asChild>
  <div style="display: flex; align-items: center; gap: 16px;">
    <UserAvatar user={JANE_DOE} size="small" />
    <UserAvatar user={JANE_DOE} size="medium-small" />
    <UserAvatar user={JANE_DOE} size="medium" />
  </div>
</Story>
