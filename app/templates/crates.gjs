import { concat, hash } from '@ember/helper';

import pageTitle from 'ember-page-title/helpers/page-title';

import CrateList from 'crates-io/components/crate-list';
import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';

<template>
  {{pageTitle 'Crates'}}

  <PageHeader @title='All Crates' @suffix={{if @controller.letter (concat "starting with '" @controller.letter "'")}} />

  <div class='results-meta'>
    <ResultsCount
      @start={{@controller.pagination.currentPageStart}}
      @end={{@controller.pagination.currentPageEnd}}
      @total={{@controller.totalItems}}
      data-test-crates-nav
    />

    <div data-test-crates-sort class='sort-by-v-center'>
      <span class='text--small'>Sort by</span>
      <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
        <sd.Option @query={{hash page=1 sort='alpha'}}>Alphabetical</sd.Option>
        <sd.Option @query={{hash page=1 sort='downloads'}}>All-Time Downloads</sd.Option>
        <sd.Option @query={{hash page=1 sort='recent-downloads'}}>Recent Downloads</sd.Option>
        <sd.Option @query={{hash page=1 sort='recent-updates'}}>Recent Updates</sd.Option>
        <sd.Option @query={{hash page=1 sort='new'}}>Newly Added</sd.Option>
      </SortDropdown>
    </div>
  </div>

  <CrateList @crates={{@controller.model}} class='list' />

  <Pagination @pagination={{@controller.pagination}} />
</template>
