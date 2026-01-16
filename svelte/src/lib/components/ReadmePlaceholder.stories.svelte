<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ReadmePlaceholder from './ReadmePlaceholder.svelte';

  const { Story } = defineMeta({
    title: 'ReadmePlaceholder',
    component: ReadmePlaceholder,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" />
