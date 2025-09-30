import { array } from '@ember/helper';
import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';

import perform from 'ember-concurrency/helpers/perform';
import pageTitle from 'ember-page-title/helpers/page-title';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

import CrateDownloadsList from 'crates-io/components/crate-downloads-list';
import LoadingSpinner from 'crates-io/components/loading-spinner';
import PageHeader from 'crates-io/components/page-header';
import dateFormatDistanceToNow from 'crates-io/helpers/date-format-distance-to-now';
import formatNum from 'crates-io/helpers/format-num';

<template>
  {{pageTitle 'Dashboard'}}

  <PageHeader class='header'>
    <h1>My Dashboard</h1>
    <div class='stats'>
      <div class='downloads'>
        {{svgJar 'download' class=(scopedClass 'header-icon')}}
        <span class='num'>{{formatNum @controller.myStats.total_downloads}}</span>
        <span class='stats-label text--small'>Total Downloads</span>
      </div>
    </div>
  </PageHeader>

  <div class='my-info'>
    <div class='my-crate-lists'>
      <div class='header'>
        <h2>
          {{svgJar 'my-packages'}}
          My Crates
        </h2>

        {{#if @controller.hasMoreCrates}}
          <LinkTo @route='user' @model={{@controller.session.currentUser.login}} class='my-crates-link'>Show all</LinkTo>
        {{/if}}
      </div>
      <CrateDownloadsList @crates={{@controller.visibleCrates}} />

      <div class='header'>
        <h2>
          {{svgJar 'following'}}
          Following
        </h2>

        {{#if @controller.hasMoreFollowing}}
          <LinkTo @route='me.following' class='followed-crates-link'>Show all</LinkTo>
        {{/if}}
      </div>
      <CrateDownloadsList @crates={{@controller.visibleFollowing}} />
    </div>

    <div class='my-feed'>
      <h2>
        {{svgJar 'latest-updates'}}
        Latest Updates
      </h2>

      <div class='feed'>
        <ul class='feed-list' data-test-feed-list>
          {{#each @controller.myFeed as |version|}}
            <li class='feed-row'>
              <LinkTo @route='crate.version' @models={{array version.crateName version.num}}>
                {{version.crateName}}
                <span class='text--small'>{{version.num}}</span>
              </LinkTo>
              <span class='feed-date text--small'>
                {{dateFormatDistanceToNow version.created_at addSuffix=true}}
              </span>
            </li>
          {{/each}}
        </ul>

        {{#if @controller.hasMore}}
          <div class='load-more'>
            <button
              type='button'
              class='load-more-button'
              disabled={{@controller.loadMoreTask.isRunning}}
              {{on 'click' (perform @controller.loadMoreTask)}}
            >
              Load More
              {{#if @controller.loadMoreTask.isRunning}}
                <LoadingSpinner />
              {{/if}}
            </button>
          </div>
        {{/if}}
      </div>
    </div>
  </div>
</template>
