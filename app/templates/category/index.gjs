import { hash } from '@ember/helper';
import { LinkTo } from '@ember/routing';

import pageTitle from 'ember-page-title/helpers/page-title';
import eq from 'ember-truth-helpers/helpers/eq';

import CrateList from 'crates-io/components/crate-list';
import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';
import formatNum from 'crates-io/helpers/format-num';

<template>
  {{pageTitle @controller.category.category ' - Categories'}}

  <PageHeader class='header'>
    <h1>
      {{#each @controller.category.parent_categories as |parent|}}<LinkTo
          @route='category'
          @model={{parent.slug}}
        >{{parent.category}}</LinkTo>::{{/each}}
      {{~@controller.category.category}}
    </h1>
  </PageHeader>

  <div>
    <p>{{@controller.category.description}}</p>
  </div>

  {{#if @controller.category.subcategories}}
    <div>
      <h2>Subcategories</h2>
      <div class='subcategories'>
        {{#each @controller.category.subcategories as |subcategory|}}
          <div class='subcategory'>
            <div>
              <LinkTo @route='category' @model={{subcategory.slug}}>{{subcategory.category}}</LinkTo>
              <span class='text--small'>
                {{formatNum subcategory.crates_cnt}}
                {{if (eq subcategory.crates_cnt 1) 'crate' 'crates'}}
              </span>
            </div>
            <div class='category-description text--small'>
              {{subcategory.description}}
            </div>
          </div>
        {{/each}}
      </div>
    </div>
  {{/if}}

  <h2>Crates</h2>
  <div class='results-meta'>
    <ResultsCount
      @start={{@controller.pagination.currentPageStart}}
      @end={{@controller.pagination.currentPageEnd}}
      @total={{@controller.totalItems}}
      data-test-category-nav
    />

    <div data-test-category-sort>
      <span class='text--small'>Sort by</span>
      <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
        <sd.Option @query={{hash sort='alpha'}}>Alphabetical</sd.Option>
        <sd.Option @query={{hash sort='downloads'}}>All-Time Downloads</sd.Option>
        <sd.Option @query={{hash sort='recent-downloads'}}>Recent Downloads</sd.Option>
        <sd.Option @query={{hash sort='recent-updates'}}>Recent Updates</sd.Option>
        <sd.Option @query={{hash sort='new'}}>Newly Added</sd.Option>
      </SortDropdown>
    </div>
  </div>

  <CrateList @crates={{@controller.model}} class='list' />

  <Pagination @pagination={{@controller.pagination}} />
</template>
