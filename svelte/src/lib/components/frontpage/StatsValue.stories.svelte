<script module>
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import StatsValue from './StatsValue.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/StatsValue',
    component: StatsValue,
    tags: ['autodocs'],
    argTypes: {
      iconClass: {
        table: {
          disable: true,
        },
      },
    },
  });
</script>

<Story name="Default" args={{ value: '1,234', label: 'Downloads', iconClass: 'i-mdi:magnify' }} />

<Story name="Large Number" args={{ value: '1,234,567', label: 'Total Downloads', iconClass: 'i-mdi:magnify' }} />
