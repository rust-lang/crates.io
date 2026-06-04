<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { nativeReplacements } from '$lib/data/native-replacements';
  import NativeReplacementBanner from './NativeReplacementBanner.svelte';

  const { Story } = defineMeta({
    title: 'NativeReplacementBanner',
    component: NativeReplacementBanner,
    tags: ['autodocs'],
  });
</script>

<Story
  name="Default"
  args={{ replacement: nativeReplacements.lazy_static }}
  parameters={{ chromatic: { disableSnapshot: true } }}
/>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Full</h1>
  <NativeReplacementBanner replacement={nativeReplacements.lazy_static} />

  <h1>Full (macro)</h1>
  <NativeReplacementBanner replacement={nativeReplacements.matches} />

  <h1>Partial</h1>
  <NativeReplacementBanner replacement={nativeReplacements.num_cpus} />

  <h1>Partial (release-notes link)</h1>
  <NativeReplacementBanner replacement={nativeReplacements.once_cell} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
