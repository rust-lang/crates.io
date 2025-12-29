<script lang="ts">
  import type { components } from '@crates-io/api-client';
  import type { HTMLAttributes } from 'svelte/elements';

  import CrateRow from './CrateRow.svelte';

  type Crate = components['schemas']['Crate'];

  interface Props extends HTMLAttributes<HTMLDivElement> {
    crates: Crate[];
  }

  let { crates, ...restProps }: Props = $props();
</script>

<!-- The extra div wrapper is needed for specificity issues with `margin` -->
<div {...restProps}>
  <ol class="list">
    {#each crates as crate, index (crate.id)}
      <li>
        <CrateRow {crate} data-test-crate-row={index} />
      </li>
    {/each}
  </ol>
</div>

<style>
  .list {
    margin: 0;
    padding: 0;
    list-style: none;

    > :global(* + *) {
      margin-top: var(--space-s);
    }
  }
</style>
