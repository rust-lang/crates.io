<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import HeroButtons from './HeroButtons.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/HeroButtons',
    component: HeroButtons,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" />
