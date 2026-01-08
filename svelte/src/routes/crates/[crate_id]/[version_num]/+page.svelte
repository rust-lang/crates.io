<script lang="ts">
  import CrateVersionPage from '$lib/components/CrateVersionPage.svelte';

  let { data } = $props();
</script>

<CrateVersionPage
  crate={data.crate}
  version={data.version}
  keywords={data.keywords}
  requestedVersion={data.requestedVersion}
  readmePromise={data.readmePromise}
/>
