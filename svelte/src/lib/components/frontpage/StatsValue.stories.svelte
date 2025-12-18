<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import SearchIcon from '$lib/assets/search.svg?component';
  import StatsValue from './StatsValue.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/StatsValue',
    component: StatsValue,
    tags: ['autodocs'],
    argTypes: {
      icon: {
        table: {
          disable: true,
        },
      },
    },
  });
</script>

<Story name="Default" args={{ value: '1,234', label: 'Downloads', icon: SearchIcon }} />

<Story name="Large Number" args={{ value: '1,234,567', label: 'Total Downloads', icon: SearchIcon }} />
