<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SettingsPage from './SettingsPage.svelte';

  const { Story } = defineMeta({
    title: 'SettingsPage',
    component: SettingsPage,
    tags: [],
  });
</script>

<Story name="Default" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <SettingsPage>
    <h2>Profile</h2>
    <p>Settings page content goes here.</p>
  </SettingsPage>
</Story>
