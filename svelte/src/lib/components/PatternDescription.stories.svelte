<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import PatternDescription from './PatternDescription.svelte';

  const { Story } = defineMeta({
    title: 'PatternDescription',
    component: PatternDescription,
    tags: ['autodocs'],
    argTypes: {
      pattern: {
        control: 'text',
      },
    },
  });
</script>

<Story name="Default" args={{ pattern: 'serde' }} parameters={{ chromatic: { disableSnapshot: true } }} />

<Story name="Combined" asChild>
  <h1>Wildcard</h1>
  <PatternDescription pattern="*" />

  <h1>Prefix Wildcard</h1>
  <PatternDescription pattern="serde-*" />

  <h1>Exact Match</h1>
  <PatternDescription pattern="serde" />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
