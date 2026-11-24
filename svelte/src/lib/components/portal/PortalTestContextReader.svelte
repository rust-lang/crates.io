<script lang="ts">
  import { getContext } from 'svelte';

  let value = getContext<string>('portal-test');
</script>

<p data-test-context>{value}</p>
