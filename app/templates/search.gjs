import { concat, hash } from '@ember/helper';
import { on } from '@ember/modifier';

import pageTitle from 'ember-page-title/helpers/page-title';

import Alert from 'crates-io/components/alert';
import CrateList from 'crates-io/components/crate-list';
import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';

<template>
  {{pageTitle @controller.pageTitle}}

  <PageHeader
    @title='Search Results'
    @suffix={{if @controller.q (concat "for '" @controller.q "'")}}
    @showSpinner={{@controller.dataTask.isRunning}}
    data-test-header
  />

  {{#if @controller.hasMultiCategoryFilter}}
    <Alert @variant='warning'>
      Support for using multiple
      <code>category:</code>
      filters is not yet implemented.
    </Alert>
  {{/if}}

  {{#if @controller.firstResultPending}}
    <h2>Loading search results...</h2>
  {{else if @controller.dataTask.lastComplete.error}}
    <p data-test-error-message>
      Unfortunately something went wrong while loading the search results. Feel free to try again, or let the
      <a href='mailto:help@crates.io'>crates.io team</a>
      know if the problem persists.
    </p>

    <button
      type='button'
      disabled={{@controller.dataTask.isRunning}}
      class='try-again-button button'
      data-test-try-again-button
      {{on 'click' @controller.fetchData}}
    >
      Try Again
    </button>
  {{else if @controller.hasItems}}
    <div class='results-meta'>
      <ResultsCount
        @start={{@controller.pagination.currentPageStart}}
        @end={{@controller.pagination.currentPageEnd}}
        @total={{@controller.totalItems}}
        data-test-search-nav
      />

      <div data-test-search-sort class='sort-by-v-center'>
        <span class='text--small'>Sort by </span>
        <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
          <sd.Option @query={{hash page=1 sort='relevance'}}>Relevance</sd.Option>
          <sd.Option @query={{hash page=1 sort='downloads'}}>All-Time Downloads</sd.Option>
          <sd.Option @query={{hash page=1 sort='recent-downloads'}}>Recent Downloads</sd.Option>
          <sd.Option @query={{hash page=1 sort='recent-updates'}}>Recent Updates</sd.Option>
          <sd.Option @query={{hash page=1 sort='new'}}>Newly Added</sd.Option>
        </SortDropdown>
      </div>
    </div>

    <CrateList @crates={{@controller.model}} class='list' />

    <Pagination @pagination={{@controller.pagination}} />
  {{else}}
    <h2>0 crates found.
      <a href='https://doc.rust-lang.org/cargo/getting-started/'>Get started</a>
      and create your own.</h2>
  {{/if}}
</template>
