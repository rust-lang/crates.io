<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import TextContent from './TextContent.svelte';

  const { Story } = defineMeta({
    title: 'TextContent',
    component: TextContent,
    tags: ['autodocs'],
  });
</script>

<Story name="Default">
  <p>This is some example text content with a <code>code snippet</code> inside.</p>
  <pre>fn main() {'{'} println!("Hello, world!"); }</pre>
</Story>

<Story name="Boxed" args={{ boxed: true }}>
  <h2>Boxed Content</h2>
  <p>This content appears in a boxed container with padding and background color.</p>
</Story>
