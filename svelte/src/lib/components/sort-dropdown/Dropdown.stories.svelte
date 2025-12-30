<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import * as SortDropdown from './index';
  import { Root } from './index';

  const { Story } = defineMeta({
    title: 'SortDropdown',
    component: Root,
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
          height: '200px',
        },
      },
    },
  });
</script>

<Story name="Default" args={{ current: 'All-Time Downloads' }}>
  <SortDropdown.Option query={{ sort: 'downloads' }}>All-Time Downloads</SortDropdown.Option>
  <SortDropdown.Option query={{ sort: 'recent-downloads' }}>Recent Downloads</SortDropdown.Option>
  <SortDropdown.Option query={{ sort: 'recent-updates' }}>Recent Updates</SortDropdown.Option>
  <SortDropdown.Option query={{ sort: 'new' }}>Newly Added</SortDropdown.Option>
</Story>
