<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import * as SideMenu from './index';

  const { Story } = defineMeta({
    title: 'SideMenu',
    component: SideMenu.Root,
    tags: ['autodocs'],
    argTypes: {
      children: { table: { disable: true } },
    },
  });
</script>

<Story name="Default" asChild>
  <SideMenu.Root>
    <SideMenu.Item href="#profile" active>Profile</SideMenu.Item>
    <SideMenu.Item href="#tokens" active={false}>API Tokens</SideMenu.Item>
  </SideMenu.Root>
</Story>
