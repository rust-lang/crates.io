<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ResultsCount from './ResultsCount.svelte';

  const { Story } = defineMeta({
    title: 'ResultsCount',
    component: ResultsCount,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" args={{ start: 1, end: 10, total: 1234 }} />

<Story name="with custom name" args={{ start: 21, end: 30, total: 567, name: 'keywords' }} />

<Story name="single page" args={{ start: 1, end: 5, total: 5 }} />
