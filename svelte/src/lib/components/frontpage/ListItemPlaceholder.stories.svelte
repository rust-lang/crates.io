<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ListItemPlaceholder from './ListItemPlaceholder.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/ListItemPlaceholder',
    component: ListItemPlaceholder,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" />

<Story name="With Subtitle" args={{ withSubtitle: true }} />

<Story name="With Trailing" args={{ withTrailing: true }} />

<Story name="With Subtitle And Trailing" args={{ withSubtitle: true, withTrailing: true }} />
