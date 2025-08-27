import { on } from '@ember/modifier';

import link_ from 'ember-link/helpers/link';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import and from 'ember-truth-helpers/helpers/and';
import not from 'ember-truth-helpers/helpers/not';

import CopyButton from 'crates-io/components/copy-button';
import Tooltip from 'crates-io/components/tooltip';
import dateFormatDistanceToNow from 'crates-io/helpers/date-format-distance-to-now';
import dateFormatIso from 'crates-io/helpers/date-format-iso';
import formatNum from 'crates-io/helpers/format-num';
import truncateText from 'crates-io/helpers/truncate-text';

<template>
  <div data-test-crate-row ...attributes class='crate-row'>
    <div class='description-box'>
      <div class='crate-spec'>
        {{#let (link_ 'crate' @crate.id) as |l|}}
          <a href={{l.url}} class='name' data-test-crate-link {{on 'click' l.transitionTo}}>
            {{@crate.name}}
          </a>
        {{/let}}
        {{#if (and @crate.default_version (not @crate.yanked))}}
          <span class='version' data-test-version>v{{@crate.default_version}}</span>
          <CopyButton
            @copyText='{{@crate.name}} = &quot;{{@crate.default_version}}&quot;'
            title='Copy Cargo.toml snippet to clipboard'
            class='copy-button button-reset'
            data-test-copy-toml-button
          >
            {{svgJar 'copy' alt='Copy Cargo.toml snippet to clipboard'}}
          </CopyButton>
        {{/if}}
      </div>
      <div class='description text--small' data-test-description>
        {{truncateText @crate.description}}
      </div>
    </div>
    <div class='stats'>
      <div class='downloads' data-test-downloads>
        {{svgJar 'download' class=(scopedClass 'download-icon')}}
        <span>
          <span>
            All-Time:
            <Tooltip @text='Total number of downloads' />
          </span>
          {{formatNum @crate.downloads}}
        </span>
      </div>
      <div class='recent-downloads' data-test-recent-downloads>
        {{svgJar 'download' class=(scopedClass 'download-icon')}}
        <span>
          <span>
            Recent:
            <Tooltip @text='Downloads in the last 90 days' />
          </span>
          {{formatNum @crate.recent_downloads}}
        </span>
      </div>
      <div class='updated-at'>
        {{svgJar 'latest-updates' height='32' width='32'}}
        <span>
          <span>
            Updated:
            <Tooltip @text='The last time the crate was updated' />
          </span>
          <time datetime='{{dateFormatIso @crate.updated_at}}' data-test-updated-at>
            {{dateFormatDistanceToNow @crate.updated_at addSuffix=true}}
            <Tooltip @text={{@crate.updated_at}} />
          </time>
        </span>
      </div>
    </div>
    <ul class='quick-links'>
      {{#if @crate.homepage}}
        <li><a href='{{@crate.homepage}}'>Homepage</a></li>
      {{/if}}
      {{#if @crate.documentation}}
        <li><a href='{{@crate.documentation}}'>Documentation</a></li>
      {{/if}}
      {{#if @crate.repository}}
        <li><a href='{{@crate.repository}}'>Repository</a></li>
      {{/if}}
    </ul>

  </div>
</template>
