import { hash } from '@ember/helper';
import { on } from '@ember/modifier';

import perform from 'ember-concurrency/helpers/perform';
import and from 'ember-truth-helpers/helpers/and';
import not from 'ember-truth-helpers/helpers/not';
import or from 'ember-truth-helpers/helpers/or';

import CrateHeader from 'crates-io/components/crate-header';
import LoadingSpinner from 'crates-io/components/loading-spinner';
import SortDropdown from 'crates-io/components/sort-dropdown';
import Row from 'crates-io/components/version-list/row';
import dateFormat from 'crates-io/helpers/date-format';

<template>
  <CrateHeader @crate={{@controller.crate}} />

  <div class='results-meta'>
    <span
      class='page-description text--small
        {{if (and @controller.loadMoreTask.isRunning (not @controller.sortedVersions)) "is-empty"}}'
      data-test-page-description
    >
      <strong>{{@controller.sortedVersions.length}}</strong>
      of
      <strong>{{@controller.crate.num_versions}}</strong>
      <strong>{{@controller.crate.name}}</strong>
      versions since
      {{dateFormat @controller.crate.created_at 'PPP'}}
    </span>

    <div data-test-search-sort>
      <span class='sort-by-label'>Sort by </span>
      <SortDropdown @current={{@controller.currentSortBy}} as |sd|>
        <sd.Option @query={{hash sort='date'}} data-test-date-sort>Date</sd.Option>
        <sd.Option @query={{hash sort='semver'}} data-test-semver-sort>SemVer</sd.Option>
      </SortDropdown>
    </div>
  </div>

  {{#if @controller.sortedVersions}}
    <ul class='list'>
      {{#each @controller.sortedVersions as |version|}}
        <li>
          <Row @version={{version}} @isOwner={{@controller.isOwner}} data-test-version={{version.num}} />
        </li>
      {{/each}}
    </ul>

    {{#if (or @controller.loadMoreTask.isRunning @controller.next_page)}}
      <div class='load-more'>
        <button
          type='button'
          class='load-more-button'
          data-test-id={{if @controller.loadMoreTask.isRunning 'loading' 'load-more'}}
          disabled={{@controller.loadMoreTask.isRunning}}
          {{on 'click' (perform @controller.loadMoreTask)}}
        >
          {{#if @controller.loadMoreTask.isRunning}}
            Loading...<LoadingSpinner class='loading-spinner' />
          {{else}}
            Load More
          {{/if}}
        </button>
      </div>
    {{/if}}
  {{else if @controller.loadMoreTask.isRunning}}
    <div class='loading'>
      <LoadingSpinner class='loading-spinner' />
    </div>
  {{/if}}
</template>
