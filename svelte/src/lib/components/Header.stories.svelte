<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Header from './Header.svelte';

  const { Story } = defineMeta({
    title: 'Header',
    component: Header,
    tags: ['autodocs'],
    globals: {
      backgrounds: { value: 'header' },
    },
  });
</script>

<Story name="Default" />

<Story name="Hero" args={{ hero: true }} />
