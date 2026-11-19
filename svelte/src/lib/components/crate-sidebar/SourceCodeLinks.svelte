<script lang="ts">
  import { resolve } from '$app/paths';

  import Icon from '#lib/components/Icon.svelte';
  import { crateFileUrl } from '#lib/utils/cdn.ts';

  interface Props {
    crate: string;
    version: string;
    cdnBase: string;
  }

  let { crate, version, cdnBase }: Props = $props();
</script>

<details class="details">
  <summary class="row">
    <Icon class="i-mdi:chevron-right chevron" />
    Download source code
  </summary>

  <ul class="links">
    <li>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- external CDN URL -->
      <a href={crateFileUrl(cdnBase, crate, version, 'crate')} download class="row" data-test-crate-link>
        <Icon class="i-mdi:package-variant-closed" />
        Download .crate
      </a>
    </li>
    <li>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- external CDN URL -->
      <a href={crateFileUrl(cdnBase, crate, version, 'zip')} download class="row" data-test-zip-link>
        <Icon class="i-mdi:folder-zip-outline" />
        Download .zip
      </a>
    </li>
    <li>
      <a
        href={resolve('/crates/[crate_id]/[version_num]/code/[...path]', {
          crate_id: crate,
          version_num: version,
          path: '',
        })}
        class="row"
        data-test-browse-link
      >
        <Icon class="i-mdi:file-tree-outline" />
        Browse files
      </a>
    </li>
  </ul>
</details>

<style>
  .details {
    margin-top: var(--space-2xs);
    margin-bottom: calc(-1 * var(--space-2xs));
  }

  summary {
    list-style: none;
    cursor: pointer;

    &::-webkit-details-marker {
      display: none;
    }

    :global(.chevron) {
      flex-shrink: 0;
      transition: transform var(--transition-fast);
    }
  }

  .details[open] summary :global(.chevron) {
    transform: rotate(90deg);
  }

  .links {
    list-style: none;
    margin: var(--space-3xs) 0 0;
    padding: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2xs);
    /* Keeps the text aligned with the sidebar column while the hover background extends to the left */
    margin-left: calc(-1 * var(--space-xs));
    padding: var(--space-2xs) var(--space-xs);
    border-radius: var(--border-radius-small);
    color: var(--main-color);
    text-decoration: none;
    font-size: 14px;
    line-height: 1.5;
    transition: background-color var(--transition-fast);

    &:hover {
      background: var(--main-bg-dark);
      color: var(--main-color);
    }

    /* The stacked sidebar has no room to the left of the column, so the rows span the full screen width instead */
    @media only screen and (max-width: 889px) {
      margin-inline: calc(-1 * var(--main-layout-padding));
      padding-inline: var(--main-layout-padding);
      border-radius: 0;
    }
  }
</style>
