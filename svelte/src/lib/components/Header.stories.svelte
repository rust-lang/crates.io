<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

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
</script>

<Story name="Default" />

<Story name="Hero" args={{ hero: true }} />
