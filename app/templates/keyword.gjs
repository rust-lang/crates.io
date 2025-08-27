import { hash } from '@ember/helper';

import pageTitle from 'ember-page-title/helpers/page-title';

import CrateList from 'crates-io/components/crate-list';
import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';

<template>
  {{pageTitle @model.keyword ' - Keywords'}}

  <PageHeader @title='All Crates' @suffix="for keyword '{{@model.keyword}}'" />

  <div class='results-meta'>
    <ResultsCount
      @start={{@controller.pagination.currentPageStart}}
      @end={{@controller.pagination.currentPageEnd}}
      @total={{@controller.totalItems}}
      data-test-keyword-nav
    />

    <div data-test-keyword-sort>
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

  <CrateList @crates={{@model.crates}} class='list' />

  <Pagination @pagination={{@controller.pagination}} />
</template>
