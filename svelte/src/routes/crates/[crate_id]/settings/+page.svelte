<script lang="ts">
  import { resolve } from '$app/paths';

  import CrateHeader from '$lib/components/CrateHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';

  let { data } = $props();

  let crate_id = $derived(data.crate.id);
</script>

<PageTitle title="Manage Crate Settings" />

<CrateHeader crate={data.crate} ownersPromise={data.ownersPromise} />

<!-- TODO: Owners section -->
<!-- TODO: Trusted Publishing section -->

<h2 class="header">Danger Zone</h2>
<div>
  <a href={resolve('/crates/[crate_id]/delete', { crate_id })} class="button button--red" data-test-delete-button>
    Delete this crate
  </a>
</div>

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-s);
    margin: var(--space-m) 0;
  }
</style>
