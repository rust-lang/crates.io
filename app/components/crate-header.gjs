import { service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

export default class CrateHeader extends Component {
  @service session;

  @alias('loadKeywordsTask.last.value') keywords;

  constructor() {
    super(...arguments);

    this.loadKeywordsTask.perform().catch(() => {
      // ignore all errors and just don't display keywords if the request fails
    });
  }

  get isOwner() {
    let userId = this.session.currentUser?.id;
    return this.args.crate?.hasOwnerUser(userId) ?? false;
  }

  loadKeywordsTask = task(async () => {
    return (await this.args.crate?.keywords) ?? [];
  });
}

<PageHeader class="header" data-test-heading>
  <h1 class="heading">
    <span data-test-crate-name>{{@crate.name}}</span>
    {{#if @version}}
      <small data-test-crate-version>v{{@version.num}}</small>

      {{#if @version.yanked}}
        <span class="yanked-badge" data-test-yanked>
          {{svg-jar "trash"}}
          Yanked

          <Tooltip>
            This crate has been yanked, but it is still available for download for other crates that
            may be depending on it.
          </Tooltip>
        </span>
      {{/if}}
    {{/if}}
  </h1>

  {{#if @crate.description}}
    <div class="description">
      {{@crate.description}}
    </div>
  {{/if}}

  {{#if this.keywords}}
    <ul class="keywords">
      {{#each this.keywords as |keyword|}}
        <li>
          <LinkTo @route="keyword" @model={{keyword.id}} data-test-keyword={{keyword.id}}>
            <span class="hash">#</span>{{keyword.id}}
          </LinkTo>
        </li>
      {{/each}}
    </ul>
  {{/if}}

  {{#if this.session.currentUser}}
    <FollowButton @crate={{@crate}} class="follow-button"/>
  {{/if}}
</PageHeader>

<NavTabs aria-label="{{@crate.name}} crate subpages" class="nav" as |nav|>
  <nav.Tab
    @link={{if
      @versionNum
      (link "crate.version" @crate @versionNum)
      (link "crate.index" @crate)
    }}
    data-test-readme-tab
  >
    Readme
  </nav.Tab>

  <nav.Tab @link={{link "crate.versions" @crate}} data-test-versions-tab>
    {{pluralize @crate.num_versions "Version"}}
  </nav.Tab>

  <nav.Tab
    @link={{if
      @versionNum
      (link "crate.version-dependencies" @crate @versionNum)
      (link "crate.dependencies" @crate)
    }}
    data-test-deps-tab
  >
    Dependencies
  </nav.Tab>

  <nav.Tab @link={{link "crate.reverse-dependencies" @crate}} data-test-rev-deps-tab>
    Dependents
  </nav.Tab>

  {{#if this.isOwner}}
    <nav.Tab @link={{link "crate.settings" @crate}} data-test-settings-tab>
      Settings
    </nav.Tab>
  {{/if}}
</NavTabs>