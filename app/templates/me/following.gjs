import { hash } from '@ember/helper';

import CrateList from 'crates-io/components/crate-list';
import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';

<template>
  <PageHeader @title='Followed Crates' />

  {{! TODO: reduce duplication with templates/me/crates.hbs }}

  <div class='results-meta'>
    <ResultsCount
      @start={{@controller.pagination.currentPageStart}}
      @end={{@controller.pagination.currentPageEnd}}
      @total={{@controller.totalItems}}
    />

    <div>
      <span class='text--small'>Sort by</span>
      <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
        <sd.Option @query={{hash sort='alpha'}}>Alphabetical</sd.Option>
        <sd.Option @query={{hash sort='downloads'}}>All-Time Downloads</sd.Option>
      </SortDropdown>
    </div>
  </div>

  <CrateList @crates={{@controller.model}} class='list' />

  <Pagination @pagination={{@controller.pagination}} />
</template>
