<script lang="ts">
  import CrateVersionPage from '$lib/components/CrateVersionPage.svelte';

  let { data } = $props();

  let downloadsPromise = $derived(
    data.downloadsPromise.then(downloads => ({ ...downloads, crateCreatedAt: data.crate.created_at })),
  );
</script>

<CrateVersionPage
  crate={data.crate}
  version={data.defaultVersion}
  keywords={data.keywords}
  readmePromise={data.readmePromise}
  {downloadsPromise}
/>
