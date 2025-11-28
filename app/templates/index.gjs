import { hash } from '@ember/helper';
import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';

import link_ from 'ember-link/helpers/link';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

import Item from 'crates-io/components/front-page-list/item';
import Placeholder from 'crates-io/components/front-page-list/item/placeholder';
import LoadingSpinner from 'crates-io/components/loading-spinner';
import StatsValue from 'crates-io/components/stats-value';
import formatNum from 'crates-io/helpers/format-num';
import placeholders from 'crates-io/helpers/placeholders';

<template>
  <div class='hero-buttons'>
    <a
      href='https://doc.rust-lang.org/cargo/getting-started/installation.html'
      class='hero-button button'
      data-test-install-cargo-link
    >
      {{svgJar 'download-arrow' class=(scopedClass 'icon')}}
      Install Cargo
    </a>

    <a href='https://doc.rust-lang.org/cargo/guide/' class='hero-button button'>
      {{svgJar 'flag' class=(scopedClass 'icon')}}
      Getting Started
    </a>
  </div>

  <div class='blurb'>
    <div class='intro'>
      Instantly publish your crates and install them. Use the API to interact and find out more information about
      available crates. Become a contributor and enhance the site with your work.
    </div>

    <div class='stats'>
      <StatsValue
        @label='Downloads'
        @value={{if @controller.hasData (formatNum @controller.model.num_downloads) '---,---,---'}}
        @icon='file-archive'
        class='downloads'
        data-test-total-downloads
      />
      <StatsValue
        @label='Crates in stock'
        @value={{if @controller.hasData (formatNum @controller.model.num_crates) '---,---'}}
        @icon='box'
        class='crates'
        data-test-total-crates
      />
    </div>
  </div>

  {{#if @controller.dataTask.lastComplete.error}}
    <p class='error-message' data-test-error-message>
      Unfortunately something went wrong while loading the crates.io summary data. Feel free to try again, or let the
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
      {{#if @controller.dataTask.isRunning}}
        <LoadingSpinner @theme='light' class='spinner' data-test-spinner />
      {{/if}}
    </button>
  {{else}}
    <div class='lists' data-test-lists>
      <section data-test-most-downloaded>
        <h2><LinkTo @route='crates' @query={{hash sort='downloads'}}>Most Downloaded</LinkTo></h2>
        <ol class='list' aria-busy='{{@controller.dataTask.isRunning}}'>
          {{#if @controller.dataTask.isRunning}}
            {{#each (placeholders 10)}}
              <li>
                <Placeholder />
              </li>
            {{/each}}
          {{else}}
            {{#each @controller.model.most_downloaded as |crate index|}}
              <li>
                <Item @link={{link_ 'crate' crate.id}} @title={{crate.name}} data-test-crate-link={{index}} />
              </li>
            {{/each}}
          {{/if}}
        </ol>
      </section>

      <section data-test-categories>
        <h2><LinkTo @route='categories'>Popular Categories</LinkTo></h2>
        <ul class='list' aria-busy='{{@controller.dataTask.isRunning}}'>
          {{#if @controller.dataTask.isRunning}}
            {{#each (placeholders 10)}}
              <li>
                <Placeholder @withSubtitle={{true}} />
              </li>
            {{/each}}
          {{else}}
            {{#each @controller.model.popular_categories as |category|}}
              <li>
                <Item
                  @link={{link_ 'category' category.slug}}
                  @title={{category.category}}
                  @subtitle='{{formatNum category.crates_cnt}} crates'
                />
              </li>
            {{/each}}
          {{/if}}
        </ul>
      </section>

      <section data-test-keywords>
        <h2><LinkTo @route='keywords'>Popular Keywords</LinkTo></h2>
        <ul class='list' aria-busy='{{@controller.dataTask.isRunning}}'>
          {{#if @controller.dataTask.isRunning}}
            {{#each (placeholders 10)}}
              <li>
                <Placeholder @withSubtitle={{true}} />
              </li>
            {{/each}}
          {{else}}
            {{#each @controller.model.popular_keywords as |keyword|}}
              <li>
                <Item
                  @link={{link_ 'keyword' keyword.id}}
                  @title={{keyword.id}}
                  @subtitle='{{formatNum keyword.crates_cnt}} crates'
                />
              </li>
            {{/each}}
          {{/if}}
        </ul>
      </section>

      <section data-test-new-crates>
        <h2><LinkTo @route='crates' @query={{hash sort='new'}}>New Crates</LinkTo></h2>
        <ol class='list' aria-busy='{{@controller.dataTask.isRunning}}'>
          {{#if @controller.dataTask.isRunning}}
            {{#each (placeholders 10)}}
              <li>
                <Placeholder @withSubtitle={{true}} />
              </li>
            {{/each}}
          {{else}}
            {{#each @controller.model.new_crates as |crate index|}}
              <li>
                <Item
                  @link={{link_ 'crate' crate.id}}
                  @title={{crate.name}}
                  @subtitle='v{{crate.newest_version}}'
                  data-test-crate-link={{index}}
                />
              </li>
            {{/each}}
          {{/if}}
        </ol>
      </section>

      <section data-test-just-updated>
        <h2><LinkTo @route='crates' @query={{hash sort='recent-updates'}}>Just Updated</LinkTo></h2>
        <ol class='list' aria-busy='{{@controller.dataTask.isRunning}}'>
          {{#if @controller.dataTask.isRunning}}
            {{#each (placeholders 10)}}
              <li>
                <Placeholder @withSubtitle={{true}} />
              </li>
            {{/each}}
          {{else}}
            {{#each @controller.model.just_updated as |crate index|}}
              <li>
                <Item
                  @link={{link_ 'crate.version' crate.id crate.newest_version}}
                  @title={{crate.name}}
                  @subtitle='v{{crate.newest_version}}'
                  data-test-crate-link={{index}}
                />
              </li>
            {{/each}}
          {{/if}}
        </ol>
      </section>

      <section data-test-most-recently-downloaded>
        <h2><LinkTo @route='crates' @query={{hash sort='recent-downloads'}}>Most Recent Downloads</LinkTo></h2>
        <ol class='list' aria-busy='{{@controller.dataTask.isRunning}}'>
          {{#if @controller.dataTask.isRunning}}
            {{#each (placeholders 10)}}
              <li>
                <Placeholder />
              </li>
            {{/each}}
          {{else}}
            {{#each @controller.model.most_recently_downloaded as |crate index|}}
              <li>
                <Item @link={{link_ 'crate' crate.id}} @title={{crate.name}} data-test-crate-link={{index}} />
              </li>
            {{/each}}
          {{/if}}
        </ol>
      </section>
    </div>
  {{/if}}
</template>
