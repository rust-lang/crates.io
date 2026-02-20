import { hash } from '@ember/helper';
import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import { didCancel } from 'ember-concurrency';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import eq from 'ember-truth-helpers/helpers/eq';
import not from 'ember-truth-helpers/helpers/not';
import or from 'ember-truth-helpers/helpers/or';

import CopyButton from 'crates-io/components/copy-button';
import InstallInstructions from 'crates-io/components/crate-sidebar/install-instructions';
import Link from 'crates-io/components/crate-sidebar/link';
import Edition from 'crates-io/components/edition';
import LicenseExpression from 'crates-io/components/license-expression';
import Msrv from 'crates-io/components/msrv';
import OwnersList from 'crates-io/components/owners-list';
import Tooltip from 'crates-io/components/tooltip';
import dateFormat from 'crates-io/helpers/date-format';
import dateFormatDistanceToNow from 'crates-io/helpers/date-format-distance-to-now';
import dateFormatIso from 'crates-io/helpers/date-format-iso';
import formatShortNum from 'crates-io/helpers/format-short-num';
import prettyBytes from 'crates-io/helpers/pretty-bytes';

import { simplifyUrl } from './crate-sidebar/link';

export default class CrateSidebar extends Component {
  @service playground;
  @service sentry;
  /** @type {import("../services/session").default} */
  @service session;

  get showHomepage() {
    let { repository, homepage } = this.args.crate;
    return homepage && (!repository || simplifyUrl(repository) !== simplifyUrl(homepage));
  }

  get playgroundLink() {
    let playgroundCrates = this.playground.crates;
    if (!playgroundCrates) return;

    let playgroundCrate = playgroundCrates.find(it => it.name === this.args.crate.name);
    if (!playgroundCrate) return;

    return `https://play.rust-lang.org/?edition=2021&code=use%20${playgroundCrate.id}%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60${playgroundCrate.id}%60%20crate%20here%0A%7D`;
  }

  get canHover() {
    return window?.matchMedia('(hover: hover)').matches;
  }

  constructor() {
    super(...arguments);

    // load Rust Playground crates list, if necessary
    this.playground.loadCrates().catch(error => {
      if (!(didCancel(error) || error.isServerError || error.isNetworkError)) {
        // report unexpected errors to Sentry
        this.sentry.captureException(error);
      }
    });
  }

  <template>
    <section aria-label='Crate metadata' ...attributes class='sidebar'>
      <div class='metadata'>
        <h2 class='heading'>Metadata</h2>

        <time datetime={{dateFormatIso @version.created_at}} class='date'>
          {{svgJar 'calendar'}}
          <span>
            {{dateFormatDistanceToNow @version.created_at addSuffix=true}}
            <Tooltip @text={{dateFormat @version.created_at 'PPP'}} />
          </span>
        </time>

        {{#if @version.rust_version}}
          <div class='msrv' data-test-msrv>
            {{svgJar 'rust'}}
            <Msrv @version={{@version}} />
          </div>
        {{else if @version.edition}}
          <div class='edition' data-test-edition>
            {{svgJar 'rust'}}
            <Edition @version={{@version}} />
          </div>
        {{/if}}

        {{#if @version.license}}
          <div class='license' data-test-license>
            {{svgJar 'license'}}
            <span>
              <LicenseExpression @license={{@version.license}} />
            </span>
          </div>
        {{/if}}

        {{#if @version.linecounts.total_code_lines}}
          <div class='linecount' data-test-linecounts>
            {{svgJar 'code'}}
            <span>
              {{formatShortNum @version.linecounts.total_code_lines}}
              SLoC
              <Tooltip>
                Source Lines of Code<br />
                <small>(excluding comments, integration tests and example code)</small>
              </Tooltip>
            </span>
          </div>
        {{/if}}

        {{#if @version.crate_size}}
          <div class='bytes'>
            {{svgJar 'weight'}}
            {{prettyBytes @version.crate_size}}
          </div>
        {{/if}}

        <div class='purl' data-test-purl>
          {{svgJar 'link'}}
          <CopyButton @copyText={{@version.purl}} class='button-reset purl-copy-button'>
            <span class='purl-text'>{{@version.purl}}</span>
            <Tooltip class='purl-tooltip'><strong>Package URL:</strong>
              {{@version.purl}}
              <small>(click to copy)</small></Tooltip>
          </CopyButton>
          <a
            href='https://github.com/package-url/purl-spec'
            target='_blank'
            rel='noopener noreferrer'
            class='purl-help-link'
            aria-label='Learn more'
          >
            {{svgJar 'circle-question'}}
            <Tooltip @text='Learn more about Package URLs' />
          </a>
        </div>
      </div>

      {{#unless @version.yanked}}
        <div data-test-install>
          <h2 class='heading'>Install</h2>

          <InstallInstructions
            @crate={{@crate.name}}
            @version={{@version.num}}
            @exactVersion={{@requestedVersion}}
            @hasLib={{not (eq @version.has_lib false)}}
            @binNames={{@version.bin_names}}
          />
        </div>
      {{/unless}}

      {{#if (or this.showHomepage @version.documentationLink @version.sourceLink @crate.repository)}}
        <div class='links'>
          {{#if this.showHomepage}}
            <Link @title='Homepage' @url={{@crate.homepage}} data-test-homepage-link />
          {{/if}}

          {{#if @version.documentationLink}}
            <Link @title='Documentation' @url={{@version.documentationLink}} data-test-docs-link />
          {{/if}}

          {{#if @version.sourceLink}}
            <Link @title='Browse source' @url={{@version.sourceLink}} data-test-source-link />
          {{/if}}

          {{#if @crate.repository}}
            <Link @title='Repository' @url={{@crate.repository}} data-test-repository-link />
          {{/if}}
        </div>
      {{/if}}

      <div>
        <h2 class='heading'>Owners</h2>
        <OwnersList @owners={{@crate.owners}} />
      </div>

      {{#unless @crate.categories.isPending}}
        {{#if @crate.categories.length}}
          <div>
            <h2 class='heading'>Categories</h2>
            <ul class='categories'>
              {{#each @crate.categories as |category|}}
                <li>
                  <LinkTo @route='category' @model={{category.slug}}>{{category.category}}</LinkTo>
                </li>
              {{/each}}
            </ul>
          </div>
        {{/if}}
      {{/unless}}

      <div>
        {{#if this.playgroundLink}}
          <a
            href={{this.playgroundLink}}
            target='_blank'
            rel='noopener noreferrer'
            class='playground-button button button--small'
            data-test-playground-button
          >
            Try on Rust Playground

            {{#if this.canHover}}
              <Tooltip
                @text='The top 100 crates are available on the Rust Playground for you to try out directly in your browser.'
              />
            {{/if}}
          </a>
          {{#unless this.canHover}}
            <p class='playground-help text--small' data-test-playground-help>
              The top 100 crates are available on the Rust Playground for you to try out directly in your browser.
            </p>
          {{/unless}}
        {{/if}}

        {{#if this.session.currentUser}}
          <LinkTo
            @route='support'
            @query={{hash inquire='crate-violation' crate=@crate.name}}
            data-test-id='link-crate-report'
            class='report-button button button--red button--small'
          >
            Report crate
          </LinkTo>
        {{/if}}
      </div>
    </section>
  </template>
}
