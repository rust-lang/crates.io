<script lang="ts">
  import type { operations } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import ListItem from './ListItem.svelte';
  import ListItemPlaceholder from './ListItemPlaceholder.svelte';

  type Summary = operations['get_summary']['responses']['200']['content']['application/json'];

  interface Props {
    summary?: Summary;
  }

  let { summary }: Props = $props();

  const numberFormat = new Intl.NumberFormat();
</script>

<div class="lists" data-test-lists>
  <section data-test-new-crates>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <h2><a href={`${resolve('/crates')}?sort=new`}>New Crates</a></h2>
    <ol class="list" aria-busy={Boolean(summary)}>
      {#if !summary}
        {#each { length: 10 } as _, i (i)}
          <li><ListItemPlaceholder withSubtitle /></li>
        {/each}
      {:else}
        {#each summary.new_crates as crate, index (crate.id)}
          <li>
            <ListItem
              title={crate.name}
              subtitle={`v${crate.newest_version}`}
              href={resolve('/crates/[crate_id]', { crate_id: crate.id })}
              data-test-crate-link={index}
            />
          </li>
        {/each}
      {/if}
    </ol>
  </section>

  <section data-test-most-downloaded>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <h2><a href={`${resolve('/crates')}?sort=downloads`}>Most Downloaded</a></h2>
    <ol class="list" aria-busy={Boolean(summary)}>
      {#if !summary}
        {#each { length: 10 } as _, i (i)}
          <li><ListItemPlaceholder /></li>
        {/each}
      {:else}
        {#each summary.most_downloaded as crate, index (crate.id)}
          <li>
            <ListItem
              title={crate.name}
              href={resolve('/crates/[crate_id]', { crate_id: crate.id })}
              data-test-crate-link={index}
            />
          </li>
        {/each}
      {/if}
    </ol>
  </section>

  <section data-test-just-updated>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <h2><a href={`${resolve('/crates')}?sort=recent-updates`}>Just Updated</a></h2>
    <ol class="list" aria-busy={Boolean(summary)}>
      {#if !summary}
        {#each { length: 10 } as _, i (i)}
          <li><ListItemPlaceholder withSubtitle /></li>
        {/each}
      {:else}
        {#each summary.just_updated as crate, index (crate.id)}
          <li>
            <ListItem
              title={crate.name}
              subtitle={`v${crate.newest_version}`}
              href={resolve('/crates/[crate_id]/[version_num]', {
                crate_id: crate.id,
                version_num: crate.newest_version,
              })}
              data-test-crate-link={index}
            />
          </li>
        {/each}
      {/if}
    </ol>
  </section>

  <section data-test-most-recently-downloaded>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <h2><a href={`${resolve('/crates')}?sort=recent-downloads`}>Most Recent Downloads</a></h2>
    <ol class="list" aria-busy={Boolean(summary)}>
      {#if !summary}
        {#each { length: 10 } as _, i (i)}
          <li><ListItemPlaceholder /></li>
        {/each}
      {:else}
        {#each summary.most_recently_downloaded as crate, index (crate.id)}
          <li>
            <ListItem
              title={crate.name}
              href={resolve('/crates/[crate_id]', { crate_id: crate.id })}
              data-test-crate-link={index}
            />
          </li>
        {/each}
      {/if}
    </ol>
  </section>

  <section data-test-keywords>
    <h2><a href={resolve('/keywords')}>Popular Keywords</a></h2>
    <ul class="list" aria-busy={Boolean(summary)}>
      {#if !summary}
        {#each { length: 10 } as _, i (i)}
          <li><ListItemPlaceholder withSubtitle /></li>
        {/each}
      {:else}
        {#each summary.popular_keywords as keyword (keyword.id)}
          <li>
            <ListItem
              title={keyword.id}
              subtitle={`${numberFormat.format(keyword.crates_cnt)} crates`}
              href={resolve('/keywords/[keyword_id]', { keyword_id: keyword.id })}
            />
          </li>
        {/each}
      {/if}
    </ul>
  </section>

  <section data-test-categories>
    <h2><a href={resolve('/categories')}>Popular Categories</a></h2>
    <ul class="list" aria-busy={Boolean(summary)}>
      {#if !summary}
        {#each { length: 10 } as _, i (i)}
          <li><ListItemPlaceholder withSubtitle /></li>
        {/each}
      {:else}
        {#each summary.popular_categories as category (category.id)}
          <li>
            <ListItem
              title={category.category}
              subtitle={`${numberFormat.format(category.crates_cnt)} crates`}
              href={resolve('/categories/[category_id]', { category_id: category.slug })}
            />
          </li>
        {/each}
      {/if}
    </ul>
  </section>
</div>

<style>
  .lists {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: var(--space-s);
    padding: 0 var(--space-s);

    @media only screen and (max-width: 750px) {
      grid-template-columns: 1fr 1fr;
    }

    @media only screen and (max-width: 550px) {
      grid-template-columns: 1fr;
    }

    h2 {
      font-size: 1.05rem;

      a:not(:hover) {
        color: var(--main-color);
      }
    }
  }

  .list {
    list-style: none;
    padding: 0;

    > * + * {
      margin-top: var(--space-2xs);
    }
  }
</style>
