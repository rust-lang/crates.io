<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import CopyButton from './CopyButton.svelte';

  const { Story } = defineMeta({
    title: 'CopyButton',
    component: CopyButton,
    tags: ['autodocs'],
    argTypes: {
      children: { table: { disable: true } },
    },
  });
</script>

<Story name="Default" args={{ copyText: 'cargo add serde' }}>Click to Copy</Story>
