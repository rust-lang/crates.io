import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import { didCancel } from 'ember-concurrency';

import { simplifyUrl } from './crate-sidebar/link';

export default class CrateSidebar extends Component {
  @service notifications;
  @service playground;
  @service sentry;

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

  @action
  async copyToClipboard(text) {
    try {
      await navigator.clipboard.writeText(text);
      this.notifications.success('Copied to clipboard!');
    } catch {
      this.notifications.error('Copy to clipboard failed!');
    }
  }
}

<section
  aria-label="Crate metadata"
  ...attributes
  class='sidebar'
>
  <div class="metadata">
    <h2 class="heading">Metadata</h2>

    <div class="purl" data-test-purl>
      {{svg-jar "link"}}
      <CopyButton
        @copyText={{@version.purl}}
        class="button-reset purl-copy-button"
      >
        <span class="purl-text">{{@version.purl}}</span>
        <Tooltip class="purl-tooltip"><strong>Package URL:</strong> {{@version.purl}} <small>(click to copy)</small></Tooltip>
      </CopyButton>
      <a
        href="https://github.com/package-url/purl-spec"
        target="_blank"
        rel="noopener noreferrer"
        class="purl-help-link"
        aria-label="Learn more"
      >
        {{svg-jar "circle-question"}}
        <Tooltip @text="Learn more about Package URLs" />
      </a>
    </div>

    <time
      datetime={{date-format-iso @version.created_at}}
      class="date"
    >
      {{svg-jar "calendar"}}
      <span>
        {{date-format-distance-to-now @version.created_at addSuffix=true}}
        <Tooltip @text={{date-format @version.created_at 'PPP'}} />
      </span>
    </time>

    {{#if @version.rust_version}}
      <div class="msrv" data-test-msrv>
        {{svg-jar "rust"}}
        <Msrv @version={{@version}} />
      </div>
    {{else if @version.edition}}
      <div class="edition" data-test-edition>
        {{svg-jar "rust"}}
        <Edition @version={{@version}} />
      </div>
    {{/if}}

    {{#if @version.license}}
      <div class="license" data-test-license>
        {{svg-jar "license"}}
        <span>
          <LicenseExpression @license={{@version.license}} />
        </span>
      </div>
    {{/if}}

    {{#if @version.crate_size}}
      <div class="bytes">
        {{svg-jar "weight"}}
        {{pretty-bytes @version.crate_size}}
      </div>
    {{/if}}
  </div>

  {{#unless @version.yanked}}
    <div data-test-install>
      <h2 class="heading">Install</h2>

      <CrateSidebar::InstallInstructions
        @crate={{@crate.name}}
        @version={{@version.num}}
        @exactVersion={{@requestedVersion}}
        @hasLib={{not (eq @version.has_lib false)}}
        @binNames={{@version.bin_names}}
      />
    </div>
  {{/unless}}

  {{#if (or this.showHomepage @version.documentationLink @crate.repository)}}
    <div class="links">
      {{#if this.showHomepage}}
        <CrateSidebar::Link
          @title="Homepage"
          @url={{@crate.homepage}}
          data-test-homepage-link
        />
      {{/if}}

      {{#if @version.documentationLink}}
        <CrateSidebar::Link
          @title="Documentation"
          @url={{@version.documentationLink}}
          data-test-docs-link
        />
      {{/if}}

      {{#if @crate.repository}}
        <CrateSidebar::Link
          @title="Repository"
          @url={{@crate.repository}}
          data-test-repository-link
        />
      {{/if}}
    </div>
  {{/if}}

  <div>
    <h2 class="heading">Owners</h2>
    <OwnersList @owners={{@crate.owners}} />
  </div>

  {{#unless @crate.categories.isPending}}
    {{#if @crate.categories.length}}
      <div>
        <h2 class="heading">Categories</h2>
        <ul class="categories">
          {{#each @crate.categories as |category|}}
            <li>
              <LinkTo @route="category" @model={{category.slug}}>{{category.category}}</LinkTo>
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
        target="_blank"
        rel="noopener noreferrer"
        class="playground-button button button--small"
        data-test-playground-button
      >
        Try on Rust Playground

        {{#if this.canHover}}
          <Tooltip
            @text="The top 100 crates are available on the Rust Playground for you to try out directly in your browser." />
        {{/if}}
      </a>
      {{#unless this.canHover}}
        <p class="playground-help text--small" data-test-playground-help>
          The top 100 crates are available on the Rust Playground for you to
          try out directly in your browser.
        </p>
      {{/unless}}
    {{/if}}

    <LinkTo
      @route="support"
      @query={{hash inquire="crate-violation" crate=@crate.name}}
      data-test-id="link-crate-report"
      class="report-button button button--red button--small"
    >
      Report crate
    </LinkTo>
  </div>
</section>