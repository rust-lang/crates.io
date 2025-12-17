<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import ErrorState from './ErrorState.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/ErrorState',
    component: ErrorState,
    tags: ['autodocs'],
  });
</script>

<Story name="Default" args={{ isLoading: false, onRetry: () => alert('Retry clicked!') }} />

<Story name="Loading" args={{ isLoading: true }} />
