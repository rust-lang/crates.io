import { action } from '@ember/object';
import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { didCancel, task } from 'ember-concurrency';
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
import { AjaxError } from 'crates-io/utils/ajax';

export default class CrateVersionPage extends Component {
  @service mermaid;
  @service sentry;
  @service session;

  @tracked stackedGraph = true;

  constructor() {
    super(...arguments);

    waitForPromise(this.loadReadmeTask.perform()).catch(() => {
      // ignored
    });
    waitForPromise(this.loadDownloadsTask.perform()).catch(() => {
      // ignored
    });

    waitForPromise(this.args.crate.loadOwnersTask.perform()).catch(() => {
      // ignored
    });

    this.args.version.loadDocsStatusTask.perform().catch(error => {
      // report unexpected errors to Sentry and ignore `ajax()` errors
      if (!didCancel(error) && !(error instanceof AjaxError)) {
        this.sentry.captureException(error);
      }
    });
  }

  get downloadsContext() {
    return this.args.requestedVersion ? this.args.version : this.args.crate;
  }

  get isOwner() {
    let userId = this.session.currentUser?.id;
    return this.args.crate.hasOwnerUser(userId);
  }

  get readme() {
    return this.loadReadmeTask.last?.value;
  }

  get downloads() {
    return this.loadDownloadsTask.last?.value;
  }

  @action setStackedGraph() {
    this.stackedGraph = true;
  }

  @action setUnstackedGraph() {
    this.stackedGraph = false;
  }

  loadReadmeTask = task(async () => {
    let version = this.args.version;

    let readme = version.loadReadmeTask.lastSuccessful
      ? version.loadReadmeTask.lastSuccessful.value
      : await version.loadReadmeTask.perform();

    // If the README contains `language-mermaid` we ensure that the `mermaid` library has loaded before we continue
    if (readme && readme.includes('language-mermaid') && !this.mermaid.loadTask.lastSuccessful?.value) {
      try {
        await this.mermaid.loadTask.perform();
      } catch (error) {
        // If we failed to load the library due to network issues, it is not the end of the world, and we just log
        // the error to the console.
        console.error(error);
      }
    }

    if (typeof document !== 'undefined') {
      setTimeout(() => {
        let e = new CustomEvent('hashchange');
        window.dispatchEvent(e);
      });
    }

    return readme;
  });

  loadDownloadsTask = task(async () => {
    let downloads = await this.downloadsContext.version_downloads;
    return downloads;
  });

  <template>
    {{pageTitle @crate.name}}

    <CrateHeader @crate={{@crate}} @version={{@version}} @versionNum={{@requestedVersion}} />

    <div class='crate-info'>
      <div class='docs' data-test-docs>
        {{#if this.loadReadmeTask.isRunning}}
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
        {{else if this.readme}}
          <article aria-label='Readme' data-test-readme>
            <RenderedHtml @html={{this.readme}} class='readme' />
          </article>
        {{else if this.loadReadmeTask.last.error}}
          <div class='readme-error' data-test-readme-error>
            Failed to load
            <code>README</code>
            file for
            {{@crate.name}}
            v{{@version.num}}

            <button
              type='button'
              class='retry-button button'
              data-test-retry-button
              {{on 'click' (perform this.loadReadmeTask)}}
            >
              Retry
            </button>
          </div>
        {{else}}
          <div class='no-readme' data-test-no-readme>
            {{@crate.name}}
            v{{@version.num}}
            appears to have no
            <code>README.md</code>
            file
          </div>
        {{/if}}
      </div>

      <CrateSidebar @crate={{@crate}} @version={{@version}} @requestedVersion={{@requestedVersion}} class='sidebar' />
    </div>

    <div class='crate-downloads'>
      <div class='stats'>
        {{#if this.downloadsContext.num}}
          <h3 data-test-crate-stats-label>
            Stats Overview for
            {{this.downloadsContext.num}}
            <LinkTo @route='crate' @model={{@crate}}>(see all)</LinkTo>
          </h3>

        {{else}}
          <h3 data-test-crate-stats-label>Stats Overview</h3>
        {{/if}}
        <div class='stat'>
          <span class='num'>
            {{svgJar 'download'}}
            <span class='num__align'>{{formatNum this.downloadsContext.downloads}}</span>
          </span>
          <span class='text--small'>Downloads all time</span>
        </div>
        <div class='stat'>
          <span class='num'>
            {{svgJar 'crate'}}
            <span class='num__align'>{{@crate.num_versions}}</span>
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
                {{#if this.stackedGraph}}
                  Stacked
                {{else}}
                  Unstacked
                {{/if}}
              </span>
            </dd.Trigger>
            <dd.Menu as |menu|>
              <menu.Item>
                <button type='button' class='dropdown-button' {{on 'click' this.setStackedGraph}}>
                  Stacked
                </button>
              </menu.Item>
              <menu.Item>
                <button type='button' class='dropdown-button' {{on 'click' this.setUnstackedGraph}}>
                  Unstacked
                </button>
              </menu.Item>
            </dd.Menu>
          </Dropdown>
        </div>
        <DownloadGraph @data={{this.downloads}} @stacked={{this.stackedGraph}} class='graph-data' />
      </div>
    </div>
  </template>
}
