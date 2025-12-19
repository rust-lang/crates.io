<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import PageHeader from './PageHeader.svelte';

  const { Story } = defineMeta({
    title: 'PageHeader',
    component: PageHeader,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" args={{ title: 'Usage Policy' }} />

<Story name="with suffix" args={{ title: 'All Crates', suffix: "for keyword 'web'" }} />

<Story name="with spinner" args={{ title: 'Search Results', suffix: "for 'web'", showSpinner: true }} />

<Story name="with custom content">custom content</Story>
