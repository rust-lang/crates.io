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
  {{pageTitle 'Keywords'}}

  <PageHeader @title='All Keywords' />

  <div class='results-meta'>
    <ResultsCount
      @start={{@controller.pagination.currentPageStart}}
      @end={{@controller.pagination.currentPageEnd}}
      @total={{@controller.totalItems}}
      data-test-keywords-nav
    />

    <div data-test-keywords-sort class='sort-by-v-center'>
      <span class='text--small'>Sort by</span>
      <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
        <sd.Option @query={{hash sort='alpha'}}>Alphabetical</sd.Option>
        <sd.Option @query={{hash sort='crates'}}># Crates</sd.Option>
      </SortDropdown>
    </div>
  </div>

  <div class='list'>
    {{#each @controller.model as |keyword|}}
      <div class='row' data-test-keyword={{keyword.id}}>
        <LinkTo @route='keyword' @model={{keyword.id}}>{{keyword.id}}</LinkTo>
        <span class='text--small' data-test-count>
          {{formatNum keyword.crates_cnt}}
          {{if (eq keyword.crates_cnt 1) 'crate' 'crates'}}
        </span>
      </div>
    {{/each}}
  </div>

  <Pagination @pagination={{@controller.pagination}} />
</template>
