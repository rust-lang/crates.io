<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Edition from './Edition.svelte';

  const { Story } = defineMeta({
    title: 'crate-sidebar/Edition',
    component: Edition,
    tags: ['autodocs'],
    argTypes: {
      edition: {
        control: 'select',
        options: ['2015', '2018', '2021', '2024'],
      },
    },
  });
</script>

<Story name="Edition 2015" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Edition edition="2015" />
  </div>
</Story>

<Story name="Edition 2018" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Edition edition="2018" />
  </div>
</Story>

<Story name="Edition 2021" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Edition edition="2021" />
  </div>
</Story>

<Story name="Edition 2024" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Edition edition="2024" />
  </div>
</Story>
