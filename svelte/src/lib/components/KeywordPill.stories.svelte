<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import KeywordPill from './KeywordPill.svelte';

  const { Story } = defineMeta({
    title: 'KeywordPill',
    component: KeywordPill,
    tags: ['autodocs'],
  });
</script>

<Story name="Combined" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <h1>Default</h1>
  <KeywordPill keyword="serde" />

  <h1>Long keyword</h1>
  <KeywordPill keyword="internationalization" />
</Story>

<style>
  h1 {
    margin: 1rem 0 0.25rem;
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
  }
</style>
