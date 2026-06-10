<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import UnmaintainedBanner from './UnmaintainedBanner.svelte';

  const { Story } = defineMeta({
    title: 'UnmaintainedBanner',
    component: UnmaintainedBanner,
    tags: ['autodocs'],
    args: {
      unmaintained: {
        id: 'RUSTSEC-2021-0139',
        url: 'https://rustsec.org/advisories/RUSTSEC-2021-0139.html',
      },
    },
  });
</script>

<Story name="Default" />
