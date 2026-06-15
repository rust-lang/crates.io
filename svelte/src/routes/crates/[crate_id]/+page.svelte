<script lang="ts">
  import CrateVersionPage from '$lib/components/CrateVersionPage.svelte';
  import { nativeReplacements } from '$lib/data/native-replacements';

  let { data } = $props();

  let downloadsPromise = $derived(
    data.downloadsPromise.then(downloads => ({ ...downloads, crateCreatedAt: data.crate.created_at })),
  );
</script>

<CrateVersionPage
  crate={data.crate}
  version={data.defaultVersion}
  categories={data.categories}
  keywords={data.keywords}
  ownersPromise={data.ownersPromise}
  readmePromise={data.readmePromise}
  playgroundCratesPromise={data.playgroundCratesPromise}
  docsRsStatusPromise={data.docsRsStatusPromise}
  {downloadsPromise}
  nativeReplacement={nativeReplacements[data.crate.name]}
/>
