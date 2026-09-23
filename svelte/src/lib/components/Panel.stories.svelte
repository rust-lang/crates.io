<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Panel from './Panel.svelte';

  const { Story } = defineMeta({
    title: 'Panel',
    component: Panel,
    tags: ['autodocs'],
    argTypes: {
      children: { table: { disable: true } },
    },
  });
</script>

<Story name="Default" parameters={{ chromatic: { disableSnapshot: true } }}>
  <Panel class="p-m">Panel content</Panel>
</Story>
