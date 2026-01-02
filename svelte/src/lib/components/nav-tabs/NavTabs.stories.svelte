<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import * as NavTabs from './index';

  const { Story } = defineMeta({
    title: 'NavTabs',
    component: NavTabs.Root,
    tags: ['autodocs'],
    argTypes: {
      children: { table: { disable: true } },
    },
  });
</script>

<Story name="Default" asChild>
  <NavTabs.Root>
    <NavTabs.Tab href="#readme" active={false}>Readme</NavTabs.Tab>
    <NavTabs.Tab href="#versions" active={true}>Versions</NavTabs.Tab>
    <NavTabs.Tab href="#dependencies" active={false}>Dependencies</NavTabs.Tab>
    <NavTabs.Tab href="#dependents" active={false}>Dependents</NavTabs.Tab>
  </NavTabs.Root>
</Story>
