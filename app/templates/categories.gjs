import { hash } from '@ember/helper';
import { LinkTo } from '@ember/routing';

import pageTitle from 'ember-page-title/helpers/page-title';
import eq from 'ember-truth-helpers/helpers/eq';

import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';
import formatNum from 'crates-io/helpers/format-num';

<template>
  {{pageTitle 'Categories'}}

  <PageHeader @title='All Categories' />

  <div class='results-meta'>
    <ResultsCount
      @start={{@controller.pagination.currentPageStart}}
      @end={{@controller.pagination.currentPageEnd}}
      @total={{@controller.totalItems}}
      data-test-categories-nav
    />

    <div data-test-categories-sort class='sort-by-v-center'>
      <span class='text--small'>Sort by</span>
      <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
        <sd.Option @query={{hash sort='alpha'}}>Alphabetical</sd.Option>
        <sd.Option @query={{hash sort='crates'}}># Crates</sd.Option>
      </SortDropdown>
    </div>
  </div>

  <div class='list'>
    {{#each @controller.model as |category|}}
      <div class='row' data-test-category={{category.slug}}>
        <div>
          <LinkTo @route='category' @model={{category.slug}} class='category-link'>
            {{~category.category~}}
          </LinkTo>
          <span class='text--small' data-test-crate-count>
            {{formatNum category.crates_cnt}}
            {{if (eq category.crates_cnt 1) 'crate' 'crates'}}
          </span>
        </div>
        <div class='description text--small'>
          {{category.description}}
        </div>
      </div>
    {{/each}}
  </div>

  <Pagination @pagination={{@controller.pagination}} />

  <div class='categories-footer'>
    Want to categorize your crate?
    <a href='https://doc.rust-lang.org/cargo/reference/manifest.html#package-metadata'>Add metadata!</a>
  </div>
</template>
