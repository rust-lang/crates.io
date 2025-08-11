import { hash } from '@ember/helper';

import svgJar from 'ember-svg-jar/helpers/svg-jar';

import CrateList from 'crates-io/components/crate-list';
import PageHeader from 'crates-io/components/page-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import SortDropdown from 'crates-io/components/sort-dropdown';
import UserAvatar from 'crates-io/components/user-avatar';
import UserLink from 'crates-io/components/user-link';

<template>
  <PageHeader class='header' data-test-heading>
    <UserAvatar @user={{@controller.model.user}} @size='medium' data-test-avatar />
    <h1 data-test-username>
      {{@controller.model.user.login}}
    </h1>
    <UserLink @user={{@controller.model.user}} class='github-link' data-test-user-link>
      {{svgJar 'github' alt='GitHub profile'}}
    </UserLink>
  </PageHeader>

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
        <sd.Option @query={{hash sort='recent-downloads'}}>Recent Downloads</sd.Option>
        <sd.Option @query={{hash sort='recent-updates'}}>Recent Updates</sd.Option>
        <sd.Option @query={{hash sort='new'}}>Newly Added</sd.Option>
      </SortDropdown>
    </div>
  </div>

  <CrateList @crates={{@controller.model.crates}} class='list' />

  <Pagination @pagination={{@controller.pagination}} />
</template>
