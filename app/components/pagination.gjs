import { concat, hash } from '@ember/helper';
import { LinkTo } from '@ember/routing';

import svgJar from 'ember-svg-jar/helpers/svg-jar';
import { eq } from 'ember-truth-helpers';

import Tooltip from './tooltip';

<template>
  <nav class='pagination' aria-label='Pagination navigation'>
    <LinkTo
      @query={{hash page=@pagination.prevPage}}
      @disabled={{eq @pagination.currentPage 1}}
      class='prev'
      rel='prev'
      title='previous page'
      data-test-pagination-prev
    >
      {{svgJar 'left-pag'}}
    </LinkTo>
    <ol>
      {{#each @pagination.pages as |page|}}
        <li>
          <LinkTo @query={{hash page=page}} title={{concat 'Go to page ' page}}>
            {{page}}
          </LinkTo>
        </li>
      {{/each}}
    </ol>
    <LinkTo
      @query={{hash page=@pagination.nextPage}}
      @disabled={{eq @pagination.currentPage @pagination.availablePages}}
      class='next'
      rel='next'
      title='next page'
      data-test-pagination-next
    >
      {{svgJar 'right-pag'}}
      {{#if (eq @pagination.currentPage @pagination.maxPages)}}
        <Tooltip>
          For performance reasons, no more pages are available. For bulk data access, please visit
          <a href='https://crates.io/data-access' target='_blank' rel='noopener noreferrer'>crates.io/data-access</a>.
        </Tooltip>
      {{/if}}
    </LinkTo>
  </nav>
</template>
