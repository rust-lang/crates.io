<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Dialog from './Dialog.svelte';

  const { Story } = defineMeta({
    title: 'Dialog',
    component: Dialog,
    tags: ['autodocs'],
    argTypes: {
      children: { table: { disable: true } },
    },
  });
</script>

<script lang="ts">
  let open = $state(false);
  let fullscreenOpen = $state(false);
</script>

<Story name="Default" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <button type="button" onclick={() => (open = true)}>Open dialog</button>

  <Dialog bind:open title="serde versions">
    <p class="content">Dialog content</p>
  </Dialog>
</Story>

<Story name="Fullscreen" asChild parameters={{ chromatic: { disableSnapshot: true } }}>
  <button type="button" onclick={() => (fullscreenOpen = true)}>Open fullscreen dialog</button>

  <Dialog bind:open={fullscreenOpen} title="serde versions" fullscreen>
    <p class="content">Dialog content</p>
  </Dialog>
</Story>

<style>
  .content {
    margin: 0;
    padding: var(--space-xs);
  }
</style>
