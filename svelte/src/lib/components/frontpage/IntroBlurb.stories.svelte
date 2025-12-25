<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import IntroBlurb from './IntroBlurb.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/IntroBlurb',
    component: IntroBlurb,
    tags: ['autodocs'],
  });
</script>

<Story name="Loading" />

<Story
  name="Loaded"
  args={{
    summary: {
      num_downloads: 123456789,
      num_crates: 150000,
      new_crates: [],
      most_downloaded: [],
      most_recently_downloaded: [],
      just_updated: [],
      popular_keywords: [],
      popular_categories: [],
    },
  }}
/>
