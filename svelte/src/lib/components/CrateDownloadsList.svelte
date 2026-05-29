<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    crates: components['schemas']['Crate'][];
  }

  let { crates }: Props = $props();

  const numberFormat = new Intl.NumberFormat();
</script>

<ul class="list">
  {#each crates as crate (crate.id)}
    <li>
      <a href={resolve('/crates/[crate_id]', { crate_id: crate.id })} class="link">
        {crate.name}
        ({crate.max_version})
        <Icon class="i-mdi:download download-icon" />
        {numberFormat.format(crate.downloads)}
        <span class="sr-only">downloads</span>
      </a>
    </li>
  {/each}
</ul>

<style>
  .list {
    list-style: none;
    padding: 0;
    margin: 0;

    > * + * {
      margin-top: var(--space-2xs);
    }
  }

  .link {
    color: light-dark(#525252, #999999);
    background-color: light-dark(#edebdd, #141413);
    font-size: 90%;
    padding: var(--space-s) var(--space-xs);
    display: flex;
    align-items: center;
  }

  .link :global(.download-icon) {
    color: #b13b89;
    margin: -0.125em;
    margin-left: auto;
    margin-right: var(--space-3xs);
    width: 1.25em;
    height: 1.25em;
  }
</style>
