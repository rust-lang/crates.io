<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Placeholder from './Placeholder.svelte';

  const { Story } = defineMeta({
    title: 'Placeholder',
    component: Placeholder,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" args={{ width: '200px', height: '20px' }} />

<Story name="Multiple Lines" asChild>
  <div style="display: flex; flex-direction: column; gap: 8px;">
    <Placeholder width="100%" height="20px" />
    <Placeholder width="80%" height="20px" />
    <Placeholder width="90%" height="20px" />
  </div>
</Story>

<Story name="Card" asChild>
  <div style="display: flex; gap: 16px; align-items: flex-start;">
    <Placeholder width="80px" height="80px" radius="8px" style="flex-shrink: 0;" />
    <div style="display: flex; flex-direction: column; gap: 8px; flex: 1;">
      <Placeholder width="60%" height="24px" />
      <Placeholder width="100%" height="16px" />
      <Placeholder width="40%" height="16px" />
    </div>
  </div>
</Story>
