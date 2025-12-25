<script lang="ts">
  import type { operations } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import ListItem from './ListItem.svelte';
  import ListSection from './ListSection.svelte';

  type Summary = operations['get_summary']['responses']['200']['content']['application/json'];
  type Crate = Summary['new_crates'][number];
  type Keyword = Summary['popular_keywords'][number];
  type Category = Summary['popular_categories'][number];

  interface Props {
    summary?: Summary;
  }

  let { summary }: Props = $props();

  const numberFormat = new Intl.NumberFormat();
</script>

<div class="lists" data-test-lists>
  <ListSection
    title="New Crates"
    href={`${resolve('/crates')}?sort=new`}
    items={summary?.new_crates}
    withSubtitle
    data-test-new-crates
  >
    {#snippet item(crate: Crate, index: number)}
      <ListItem
        title={crate.name}
        subtitle={`v${crate.newest_version}`}
        href={resolve('/crates/[crate_id]', { crate_id: crate.id })}
        data-test-crate-link={index}
      />
    {/snippet}
  </ListSection>

  <ListSection
    title="Most Downloaded"
    href={`${resolve('/crates')}?sort=downloads`}
    items={summary?.most_downloaded}
    data-test-most-downloaded
  >
    {#snippet item(crate: Crate, index: number)}
      <ListItem
        title={crate.name}
        href={resolve('/crates/[crate_id]', { crate_id: crate.id })}
        data-test-crate-link={index}
      />
    {/snippet}
  </ListSection>

  <ListSection
    title="Just Updated"
    href={`${resolve('/crates')}?sort=recent-updates`}
    items={summary?.just_updated}
    withSubtitle
    data-test-just-updated
  >
    {#snippet item(crate: Crate, index: number)}
      <ListItem
        title={crate.name}
        subtitle={`v${crate.newest_version}`}
        href={resolve('/crates/[crate_id]/[version_num]', {
          crate_id: crate.id,
          version_num: crate.newest_version,
        })}
        data-test-crate-link={index}
      />
    {/snippet}
  </ListSection>

  <ListSection
    title="Most Recent Downloads"
    href={`${resolve('/crates')}?sort=recent-downloads`}
    items={summary?.most_recently_downloaded}
    data-test-most-recently-downloaded
  >
    {#snippet item(crate: Crate, index: number)}
      <ListItem
        title={crate.name}
        href={resolve('/crates/[crate_id]', { crate_id: crate.id })}
        data-test-crate-link={index}
      />
    {/snippet}
  </ListSection>

  <ListSection
    title="Popular Keywords"
    href={resolve('/keywords')}
    items={summary?.popular_keywords}
    withSubtitle
    ordered={false}
    data-test-keywords
  >
    {#snippet item(keyword: Keyword)}
      <ListItem
        title={keyword.id}
        subtitle={`${numberFormat.format(keyword.crates_cnt)} crates`}
        href={resolve('/keywords/[keyword_id]', { keyword_id: keyword.id })}
      />
    {/snippet}
  </ListSection>

  <ListSection
    title="Popular Categories"
    href={resolve('/categories')}
    items={summary?.popular_categories}
    withSubtitle
    ordered={false}
    data-test-categories
  >
    {#snippet item(category: Category)}
      <ListItem
        title={category.category}
        subtitle={`${numberFormat.format(category.crates_cnt)} crates`}
        href={resolve('/categories/[category_id]', { category_id: category.slug })}
      />
    {/snippet}
  </ListSection>
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
  }
</style>
