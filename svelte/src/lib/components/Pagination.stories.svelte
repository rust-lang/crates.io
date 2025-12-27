<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import { calculatePagination } from '$lib/utils/pagination';
  import Pagination from './Pagination.svelte';

  const { Story } = defineMeta({
    title: 'Pagination',
    component: Pagination,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" args={{ pagination: calculatePagination(5, 10, 200) }} />

<Story name="First page" args={{ pagination: calculatePagination(1, 10, 200) }} />

<Story name="Last page" args={{ pagination: calculatePagination(20, 10, 200) }} />

<Story name="Few pages" args={{ pagination: calculatePagination(2, 10, 30) }} />
