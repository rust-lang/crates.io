<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Alert from './Alert.svelte';

  const { Story } = defineMeta({
    title: 'Alert',
    component: Alert,
    tags: ['autodocs'],
    argTypes: {
      children: { table: { disable: true } },
    },
  });
</script>

<Story name="Note" args={{ variant: 'note' }}>This is a note alert with some informational content.</Story>

<Story name="Success" args={{ variant: 'success' }}>This action completed successfully!</Story>

<Story name="Tip" args={{ variant: 'tip' }}>Here's a helpful tip for you.</Story>

<Story name="Important" args={{ variant: 'important' }}>This is an important message that requires attention.</Story>

<Story name="Warning" args={{ variant: 'warning' }}>Warning: Please review before proceeding.</Story>

<Story name="Caution" args={{ variant: 'caution' }}>Caution: This action cannot be undone.</Story>

<Story name="Without Icon" args={{ variant: 'note', hideIcon: true }}>This alert has no icon displayed.</Story>
