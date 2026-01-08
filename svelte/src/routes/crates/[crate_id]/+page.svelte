<script lang="ts">
  import CrateVersionPage from '$lib/components/CrateVersionPage.svelte';

  let { data } = $props();
</script>

<CrateVersionPage
  crate={data.crate}
  version={data.defaultVersion}
  keywords={data.keywords}
  readmePromise={data.readmePromise}
/>
