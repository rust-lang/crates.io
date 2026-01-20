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
  ownersPromise={data.ownersPromise}
  readmePromise={data.readmePromise}
  playgroundCratesPromise={data.playgroundCratesPromise}
  {downloadsPromise}
/>
