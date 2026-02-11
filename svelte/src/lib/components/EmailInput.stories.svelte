<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import NotificationDecorator from '$lib/storybook/NotificationDecorator.svelte';
  import EmailInput from './EmailInput.svelte';

  const { Story } = defineMeta({
    title: 'EmailInput',
    component: EmailInput,
    tags: [],
  });

  type AuthenticatedUser = components['schemas']['AuthenticatedUser'];

  const BASE_USER: AuthenticatedUser = {
    id: 42,
    login: 'johndoe',
    name: 'John Doe',
    avatar: 'https://avatars.githubusercontent.com/u/1234567?v=4',
    url: 'https://github.com/johndoe',
    is_admin: false,
    publish_notifications: true,
    email: 'john@example.com',
    email_verified: true,
    email_verification_sent: true,
  };

  const UNVERIFIED_USER: AuthenticatedUser = {
    ...BASE_USER,
    email_verified: false,
    email_verification_sent: true,
  };

  const UNVERIFIED_NOT_SENT_USER: AuthenticatedUser = {
    ...BASE_USER,
    email_verified: false,
    email_verification_sent: false,
  };

  const NO_EMAIL_USER: AuthenticatedUser = {
    ...BASE_USER,
    email: null,
    email_verified: false,
    email_verification_sent: false,
  };
</script>

<Story name="Combined" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <NotificationDecorator>
    <h1>Verified Email</h1>
    <EmailInput user={BASE_USER} />

    <h1>Unverified Email (Verification Sent)</h1>
    <EmailInput user={UNVERIFIED_USER} />

    <h1>Unverified Email (Verification Not Sent)</h1>
    <EmailInput user={UNVERIFIED_NOT_SENT_USER} />

    <h1>No Email</h1>
    <EmailInput user={NO_EMAIL_USER} />
  </NotificationDecorator>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
