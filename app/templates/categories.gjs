{{page-title 'Categories'}}

<PageHeader @title="All Categories"/>

<div class="results-meta">
  <ResultsCount
    @start={{this.pagination.currentPageStart}}
    @end={{this.pagination.currentPageEnd}}
    @total={{this.totalItems}}
    data-test-categories-nav
  />

  <div data-test-categories-sort class="sort-by-v-center">
    <span class="text--small">Sort by</span>
    <SortDropdown @current={{this.currentSortBy}} as |sd|>
      <sd.Option @query={{hash sort="alpha"}}>Alphabetical</sd.Option>
      <sd.Option @query={{hash sort="crates"}}># Crates</sd.Option>
    </SortDropdown>
  </div>
</div>

<div class="list">
  {{#each this.model as |category|}}
    <div class='row' data-test-category={{category.slug}}>
      <div>
        <LinkTo @route="category" @model={{category.slug}} class="category-link">
          {{~category.category~}}
        </LinkTo>
        <span class="text--small" data-test-crate-count>
          {{format-num category.crates_cnt}} {{if (eq category.crates_cnt 1) "crate" "crates"}}
        </span>
      </div>
      <div class="description text--small">
        {{ category.description }}
      </div>
    </div>
  {{/each}}
</div>

<Pagination @pagination={{this.pagination}} />

<div class='categories-footer'>
  Want to categorize your crate?
  <a href='https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata'>Add metadata!</a>
</div>
