import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';

import perform from 'ember-concurrency/helpers/perform';
import pageTitle from 'ember-page-title/helpers/page-title';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

import CrateHeader from 'crates-io/components/crate-header';
import CrateSidebar from 'crates-io/components/crate-sidebar';
import DownloadGraph from 'crates-io/components/download-graph';
import Dropdown from 'crates-io/components/dropdown';
import Placeholder from 'crates-io/components/placeholder';
import RenderedHtml from 'crates-io/components/rendered-html';
import formatNum from 'crates-io/helpers/format-num';
<template>
  {{pageTitle @controller.crate.name}}

  <CrateHeader
    @crate={{@controller.crate}}
    @version={{@controller.currentVersion}}
    @versionNum={{@controller.requestedVersion}}
  />

  <div class='crate-info'>
    <div class='docs' data-test-docs>
      {{#if @controller.loadReadmeTask.isRunning}}
        <div class='readme-spinner'>
          <Placeholder class='placeholder-title' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-subtitle' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-text' />
          <Placeholder class='placeholder-text' />
        </div>
      {{else if @controller.readme}}
        <article aria-label='Readme' data-test-readme>
          <RenderedHtml @html={{@controller.readme}} class='readme' />
        </article>
      {{else if @controller.loadReadmeTask.last.error}}
        <div class='readme-error' data-test-readme-error>
          Failed to load
          <code>README</code>
          file for
          {{@controller.crate.name}}
          v{{@controller.currentVersion.num}}

          <button
            type='button'
            class='retry-button button'
            data-test-retry-button
            {{on 'click' (perform @controller.loadReadmeTask)}}
          >
            Retry
          </button>
        </div>
      {{else}}
        <div class='no-readme' data-test-no-readme>
          {{@controller.crate.name}}
          v{{@controller.currentVersion.num}}
          appears to have no
          <code>README.md</code>
          file
        </div>
      {{/if}}
    </div>

    <CrateSidebar
      @crate={{@controller.crate}}
      @version={{@controller.currentVersion}}
      @requestedVersion={{@controller.requestedVersion}}
      class='sidebar'
    />
  </div>

  <div class='crate-downloads'>
    <div class='stats'>
      {{#if @controller.downloadsContext.num}}
        <h3 data-test-crate-stats-label>
          Stats Overview for
          {{@controller.downloadsContext.num}}
          <LinkTo @route='crate' @model={{@controller.crate}}>(see all)</LinkTo>
        </h3>

      {{else}}
        <h3 data-test-crate-stats-label>Stats Overview</h3>
      {{/if}}
      <div class='stat'>
        <span class='num'>
          {{svgJar 'download'}}
          <span class='num__align'>{{formatNum @controller.downloadsContext.downloads}}</span>
        </span>
        <span class='text--small'>Downloads all time</span>
      </div>
      <div class='stat'>
        <span class='num'>
          {{svgJar 'crate'}}
          <span class='num__align'>{{@controller.crate.num_versions}}</span>
        </span>
        <span class='text--small'>Versions published</span>
      </div>
    </div>
    <div class='graph'>
      <h4>Downloads over the last 90 days</h4>
      <div class='toggle-stacked'>
        <span class='toggle-stacked-label'>Display as </span>
        <Dropdown as |dd|>
          <dd.Trigger class='trigger'>
            <span class='trigger-label'>
              {{#if @controller.stackedGraph}}
                Stacked
              {{else}}
                Unstacked
              {{/if}}
            </span>
          </dd.Trigger>
          <dd.Menu as |menu|>
            <menu.Item>
              <button type='button' class='dropdown-button' {{on 'click' @controller.setStackedGraph}}>
                Stacked
              </button>
            </menu.Item>
            <menu.Item>
              <button type='button' class='dropdown-button' {{on 'click' @controller.setUnstackedGraph}}>
                Unstacked
              </button>
            </menu.Item>
          </dd.Menu>
        </Dropdown>
      </div>
      <DownloadGraph @data={{@controller.downloads}} @stacked={{@controller.stackedGraph}} class='graph-data' />
    </div>
  </div>
</template>
