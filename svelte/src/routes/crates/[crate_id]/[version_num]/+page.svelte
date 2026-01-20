<script lang="ts">
  import CrateVersionPage from '$lib/components/CrateVersionPage.svelte';

  let { data } = $props();

  let downloadsPromise = $derived(
    data.downloadsPromise.then(downloads => ({ ...downloads, versions: [data.version] })),
  );
</script>

<CrateVersionPage
  crate={data.crate}
  version={data.version}
  keywords={data.keywords}
  ownersPromise={data.ownersPromise}
  requestedVersion={data.requestedVersion}
  readmePromise={data.readmePromise}
  playgroundCratesPromise={data.playgroundCratesPromise}
  {downloadsPromise}
/>
