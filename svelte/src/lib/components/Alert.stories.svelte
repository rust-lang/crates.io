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

<Story name="Default" args={{ variant: 'note', hideIcon: false }} parameters={{ chromatic: { disableSnapshot: true } }}>
  This is a note alert with some informational content.
</Story>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Note</h1>
  <Alert variant="note">This is a note alert with some informational content.</Alert>

  <h1>Success</h1>
  <Alert variant="success">This action completed successfully!</Alert>

  <h1>Tip</h1>
  <Alert variant="tip">Here's a helpful tip for you.</Alert>

  <h1>Important</h1>
  <Alert variant="important">This is an important message that requires attention.</Alert>

  <h1>Warning</h1>
  <Alert variant="warning">Warning: Please review before proceeding.</Alert>

  <h1>Caution</h1>
  <Alert variant="caution">Caution: This action cannot be undone.</Alert>

  <h1>Without Icon</h1>
  <Alert variant="note" hideIcon={true}>This alert has no icon displayed.</Alert>
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
