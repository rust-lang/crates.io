<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Msrv from './Msrv.svelte';

  const { Story } = defineMeta({
    title: 'crate-sidebar/Msrv',
    component: Msrv,
    tags: ['autodocs'],
    argTypes: {
      msrv: { control: 'text' },
      edition: {
        control: 'select',
        options: [undefined, '2015', '2018', '2021', '2024'],
      },
    },
  });
</script>

<Story name="MSRV Only" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Msrv msrv="1.70.0" />
  </div>
</Story>

<Story name="With Edition 2021" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Msrv msrv="1.70.0" edition="2021" />
  </div>
</Story>

<Story name="With Edition 2024" asChild>
  <div style="display: flex; justify-content: center; padding-top: 100px;">
    <Msrv msrv="1.85.0" edition="2024" />
  </div>
</Story>
