<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ListItem from './ListItem.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/ListItem',
    component: ListItem,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" args={{ href: '#', title: 'serde' }} />

<Story
  name="With Subtitle"
  args={{ href: '#', title: 'serde', subtitle: 'A generic serialization/deserialization framework' }}
/>

<Story
  name="Long Content"
  args={{
    href: '#',
    title: 'some-very-long-crate-name-that-might-overflow-and-should-not-break-the-rendering',
    subtitle: 'An event-driven, non-blocking I/O platform for writing asynchronous applications',
  }}
/>
