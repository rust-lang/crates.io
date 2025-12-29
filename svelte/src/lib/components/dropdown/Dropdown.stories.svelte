<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import * as Dropdown from './index';

  const { Story } = defineMeta({
    title: 'Dropdown',
    component: Dropdown.Root,
    tags: ['autodocs'],
    argTypes: {
      children: {
        table: {
          disable: true,
        },
      },
    },
    parameters: {
      docs: {
        story: {
          height: '150px',
        },
      },
    },
  });
</script>

<Story name="Basic Menu" asChild>
  <Dropdown.Root>
    <Dropdown.Trigger>Options</Dropdown.Trigger>
    <Dropdown.Menu>
      <Dropdown.Item><a href="#profile">Profile</a></Dropdown.Item>
      <Dropdown.Item><a href="#settings">Settings</a></Dropdown.Item>
      <Dropdown.Item><button type="button" class="button-reset">Sign Out</button></Dropdown.Item>
    </Dropdown.Menu>
  </Dropdown.Root>
</Story>
