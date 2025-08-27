import CrateHeader from 'crates-io/components/crate-header';
import Pagination from 'crates-io/components/pagination';
import ResultsCount from 'crates-io/components/results-count';
import RevDepRow from 'crates-io/components/rev-dep-row';

<template>
  <CrateHeader @crate={{@controller.crate}} />

  {{#if @controller.model}}
    <div class='results-meta'>
      <ResultsCount
        @start={{@controller.pagination.currentPageStart}}
        @end={{@controller.pagination.currentPageEnd}}
        @total={{@controller.totalItems}}
        @name='reverse dependencies of {{@controller.crate.name}}'
      />
    </div>

    <ul class='list' data-test-list>
      {{#each @controller.model as |dependency index|}}
        <li class='row'>
          <RevDepRow @dependency={{dependency}} data-test-row={{index}} />
        </li>
      {{/each}}
    </ul>

    <Pagination @pagination={{@controller.pagination}} />
  {{else}}
    <div class='no-results'>
      This crate is not used as a dependency in any other crate on crates.io.
    </div>
  {{/if}}
</template>
