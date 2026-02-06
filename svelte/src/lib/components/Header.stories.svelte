<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SessionDecorator from '$lib/storybook/SessionDecorator.svelte';
  import Header from './Header.svelte';

  const { Story } = defineMeta({
    title: 'Header',
    component: Header,
    tags: ['autodocs'],
    globals: {
      backgrounds: { value: 'header' },
      theme: { value: 'system' },
    },
  });

  type AuthenticatedUser = components['schemas']['AuthenticatedUser'];

  const baseUser: AuthenticatedUser = {
    id: 42,
    login: 'johndoe',
    name: 'John Doe',
    avatar: 'https://avatars.githubusercontent.com/u/1234567?v=4',
    email: 'john@example.com',
    email_verified: true,
    email_verification_sent: true,
    is_admin: false,
    publish_notifications: true,
    url: 'https://github.com/johndoe',
  };

  const adminUser: AuthenticatedUser = {
    ...baseUser,
    login: 'admin',
    name: 'Admin User',
    is_admin: true,
  };
</script>

<Story name="Default" />

<Story name="Authenticated" asChild>
  <SessionDecorator user={baseUser}>
    <Header />
  </SessionDecorator>
</Story>

<Story name="Hero" args={{ hero: true }} />

<Story name="Hero (Authenticated Admin)" asChild>
  <SessionDecorator user={adminUser}>
    <Header hero />
  </SessionDecorator>
</Story>
