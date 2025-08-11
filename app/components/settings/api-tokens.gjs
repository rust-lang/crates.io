import { hash } from '@ember/helper';
import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import eq from 'ember-truth-helpers/helpers/eq';
import or from 'ember-truth-helpers/helpers/or';

import CopyButton from 'crates-io/components/copy-button';
import LoadingSpinner from 'crates-io/components/loading-spinner';
import Tooltip from 'crates-io/components/tooltip';
import dateFormatDistanceToNow from 'crates-io/helpers/date-format-distance-to-now';
import isClipboardSupported from 'crates-io/helpers/is-clipboard-supported';

import { patternDescription, scopeDescription } from '../../utils/token-scopes';

export default class ApiTokens extends Component {
  <template>
    <div class='me-subheading'>
      <h2>API Tokens</h2>
      <div class='right'>
        <LinkTo @route='settings.tokens.new' class='button' data-test-new-token-button>
          New Token
        </LinkTo>
      </div>
    </div>

    <p class='explainer'>
      You can use the API tokens generated on this page to run
      <a href='https://doc.rust-lang.org/cargo/'>cargo</a>
      commands that need write access to crates.io. If you want to publish your own crates then this is required.
    </p>

    <p class='explainer'>
      To prevent keys being silently leaked they are stored on crates.io in hashed form. This means you can only
      download keys when you first create them. If you have old unused keys you can safely delete them and create a new
      one.
    </p>

    <p class='explainer'>
      To use an API token, run
      <a href='https://doc.rust-lang.org/cargo/commands/cargo-login.html'><code>cargo login</code></a>
      on the command line and paste the key when prompted. This will save it to a
      <a href='https://doc.rust-lang.org/cargo/reference/config.html#credentials'>local credentials file</a>. For CI
      systems you can use the
      <a href='https://doc.rust-lang.org/cargo/reference/config.html?highlight=CARGO_REGISTRY_TOKEN#credentials'><code
        >CARGO_REGISTRY_TOKEN</code></a>
      environment variable, but make sure that the token stays secret!
    </p>

    {{#if this.sortedTokens}}
      <ul role='list' class='token-list'>
        {{#each this.sortedTokens as |token|}}
          <li class='row {{if token.isExpired "expired"}}' data-test-api-token={{or token.id true}}>
            <h3 class='name' data-test-name>
              {{token.name}}
            </h3>

            {{#if (or token.endpoint_scopes token.crate_scopes)}}
              <div class='scopes text--small'>
                {{#if token.endpoint_scopes}}
                  <div class='endpoint-scopes' data-test-endpoint-scopes>
                    Scopes:

                    {{#each (this.listToParts token.endpoint_scopes) as |part|~}}
                      {{#if (eq part.type 'element')}}
                        <strong>{{part.value}}<Tooltip @text={{this.scopeDescription part.value}} /></strong>
                      {{~else~}}
                        {{part.value}}
                      {{/if}}
                    {{~/each}}
                  </div>
                {{/if}}

                {{#if token.crate_scopes}}
                  <div class='crate-scopes' data-test-crate-scopes>
                    Crates:

                    {{#each (this.listToParts token.crate_scopes) as |part|~}}
                      {{#if (eq part.type 'element')}}
                        <strong>{{part.value}}<Tooltip @text={{this.patternDescription part.value}} /></strong>
                      {{~else~}}
                        {{part.value}}
                      {{/if}}
                    {{~/each}}
                  </div>
                {{/if}}
              </div>
            {{/if}}

            <div class='metadata text--small'>
              <div title={{token.last_used_at}} class='last-used-at' data-test-last-used-at>
                {{#if token.last_used_at}}
                  Last used
                  {{dateFormatDistanceToNow token.last_used_at addSuffix=true}}
                {{else}}
                  Never used
                {{/if}}
              </div>

              <div title={{token.created_at}} class='created-at' data-test-created-at>
                Created
                {{dateFormatDistanceToNow token.created_at addSuffix=true}}
              </div>

              {{#if token.expired_at}}
                <div title={{token.expired_at}} class='expired-at' data-test-expired-at>
                  {{if token.isExpired 'Expired' 'Expires'}}
                  {{dateFormatDistanceToNow token.expired_at addSuffix=true}}
                </div>
              {{/if}}
            </div>

            {{#if token.token}}
              <div class='new-token'>
                <div class='new-token-explainer'>
                  Make sure to copy your API token now. You won’t be able to see it again!
                </div>

                <div class='token-display'>
                  <span class='token-value' data-test-token>{{token.token}}</span>

                  {{#if (isClipboardSupported)}}
                    <CopyButton @copyText={{token.token}} class='copy-button button-reset'>
                      <span class='sr-only'>Copy</span>
                      {{svgJar 'copy' aria-hidden='true' class=(scopedClass 'copy-button-icon')}}
                    </CopyButton>
                  {{/if}}
                </div>
              </div>
            {{/if}}

            <div class='actions'>
              <LinkTo
                @route='settings.tokens.new'
                @query={{hash from=token.id}}
                class='regenerate-button button button--small'
                data-test-regenerate-token-button
              >
                Regenerate
              </LinkTo>
              {{#unless token.isExpired}}
                <button
                  type='button'
                  class='revoke-button button button--tan button--small'
                  disabled={{token.isSaving}}
                  data-test-revoke-token-button
                  {{on 'click' (perform this.revokeTokenTask token)}}
                >
                  Revoke
                </button>
                {{#if token.isSaving}}
                  <LoadingSpinner class='spinner' data-test-saving-spinner />
                {{/if}}
              {{/unless}}
            </div>
          </li>
        {{/each}}
      </ul>
    {{else}}
      <div class='empty-state'>
        <div class='empty-state-label'>
          You have not generated any API tokens yet.
        </div>

        <LinkTo
          @route='settings.tokens.new'
          class='empty-state-button button button--small'
          data-test-empty-state-button
        >
          New Token
        </LinkTo>
      </div>
    {{/if}}
  </template>
  @service store;
  @service notifications;
  @service router;

  scopeDescription = scopeDescription;
  patternDescription = patternDescription;

  get sortedTokens() {
    return this.args.tokens
      .filter(t => !t.isNew)
      .sort((a, b) => {
        // Expired tokens are always shown after active ones.
        if (a.isExpired && !b.isExpired) {
          return 1;
        } else if (b.isExpired && !a.isExpired) {
          return -1;
        }

        // Otherwise, sort normally based on creation time.
        return a.created_at < b.created_at ? 1 : -1;
      });
  }

  listToParts(list) {
    // We hardcode `en-US` here because the rest of the interface text is also currently displayed only in English.
    return new Intl.ListFormat('en-US').formatToParts(list);
  }

  @action startNewToken() {
    this.router.transitionTo('settings.tokens.new');
  }

  revokeTokenTask = task(async token => {
    try {
      await token.destroyRecord();

      let index = this.args.tokens.indexOf(token);
      if (index !== -1) {
        this.args.tokens.splice(index, 1);
      }
    } catch (error) {
      let detail = error.errors?.[0]?.detail;

      let msg =
        detail && !detail.startsWith('{')
          ? `An error occurred while revoking this token, ${detail}`
          : 'An unknown error occurred while revoking this token';

      this.notifications.error(msg);
    }
  });
}
