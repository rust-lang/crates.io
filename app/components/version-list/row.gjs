import { fn } from '@ember/helper';
import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { LinkTo } from '@ember/routing';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import and from 'ember-truth-helpers/helpers/and';
import eq from 'ember-truth-helpers/helpers/eq';
import or from 'ember-truth-helpers/helpers/or';

import Dropdown from 'crates-io/components/dropdown';
import Edition from 'crates-io/components/edition';
import LicenseExpression from 'crates-io/components/license-expression';
import Msrv from 'crates-io/components/msrv';
import PrivilegedAction from 'crates-io/components/privileged-action';
import Tooltip from 'crates-io/components/tooltip';
import UserAvatar from 'crates-io/components/user-avatar';
import YankButton from 'crates-io/components/yank-button';
import dateFormat from 'crates-io/helpers/date-format';
import dateFormatDistanceToNow from 'crates-io/helpers/date-format-distance-to-now';
import dateFormatIso from 'crates-io/helpers/date-format-iso';
import prettyBytes from 'crates-io/helpers/pretty-bytes';

export default class VersionRow extends Component {
  @tracked focused = false;

  get features() {
    let features = this.args.version.featureList;
    let list = features.slice(0, 15);
    let more = features.length - list.length;
    return { list, more };
  }

  @action setFocused(value) {
    this.focused = value;
  }

  <template>
    <div
      ...attributes
      class='row
        {{if @version.isHighestOfReleaseTrack "latest"}}
        {{if @version.yanked "yanked"}}
        {{if @version.isPrerelease "prerelease"}}
        {{if this.focused "focused"}}
        '
    >
      <div class='version'>
        <div class='release-track' data-test-release-track>
          {{#if @version.yanked}}
            {{svgJar 'trash'}}
          {{else if @version.invalidSemver}}
            ?
          {{else}}
            {{@version.releaseTrack}}
          {{/if}}

          <Tooltip @side='right' class='rt-tooltip' data-test-release-track-title>
            {{#if @version.yanked}}
              This version was
              <span class='rt-yanked'>yanked</span>
            {{else if @version.invalidSemver}}
              Failed to parse version
              {{@version.num}}
            {{else}}
              Release Track:
              {{@version.releaseTrack}}
              {{#if (or @version.isPrerelease @version.isHighestOfReleaseTrack)}}
                ({{#if @version.isPrerelease}}<span class='rt-prerelease'>prerelease</span>{{/if}}{{#if
                  (and @version.isPrerelease @version.isHighestOfReleaseTrack)
                }}, {{/if}}{{#if @version.isHighestOfReleaseTrack}}<span class='rt-latest'>latest</span>{{/if}})
              {{/if}}
            {{/if}}
          </Tooltip>
        </div>

        <LinkTo
          @route='crate.version'
          @model={{@version.num}}
          class='num-link'
          {{on 'focusin' (fn this.setFocused true)}}
          {{on 'focusout' (fn this.setFocused false)}}
          data-test-release-track-link
        >
          {{@version.num}}
        </LinkTo>
      </div>

      <div class='metadata'>
        <div class='metadata-row'>
          {{#if @version.published_by}}
            <span class='publisher'>
              by
              <LinkTo @route='user' @model={{@version.published_by.login}}>
                <UserAvatar @user={{@version.published_by}} class='avatar' />
                {{or @version.published_by.name @version.published_by.login}}
              </LinkTo>
            </span>
          {{else if @version.trustpubPublisher}}
            <span local-class='publisher trustpub'>
              via
              {{#if @version.trustpubUrl}}
                <a href={{@version.trustpubUrl}} target='_blank' rel='nofollow noopener noreferrer'>
                  {{#if (eq @version.trustpub_data.provider 'github')}}
                    {{svgJar 'github'}}
                  {{else if (eq @version.trustpub_data.provider 'gitlab')}}
                    {{svgJar 'gitlab'}}
                  {{/if}}
                  {{@version.trustpubPublisher}}
                </a>
              {{else}}
                {{#if (eq @version.trustpub_data.provider 'github')}}
                  {{svgJar 'github'}}
                {{else if (eq @version.trustpub_data.provider 'gitlab')}}
                  {{svgJar 'gitlab'}}
                {{/if}}
                {{@version.trustpubPublisher}}
              {{/if}}
            </span>
          {{/if}}

          <time datetime={{dateFormatIso @version.created_at}} class='date {{if @version.isNew "new"}}'>
            {{svgJar 'calendar'}}
            {{dateFormatDistanceToNow @version.created_at addSuffix=true}}

            <Tooltip class='tooltip'>
              {{dateFormat @version.created_at 'PPP'}}
              {{#if @version.isNew}}
                (<span class='new'>new</span>)
              {{/if}}
            </Tooltip>
          </time>
        </div>

        {{#if (or @version.crate_size @version.license @version.featureList)}}
          <div class='metadata-row'>
            {{#if @version.rust_version}}
              <span class='msrv'>
                {{svgJar 'rust'}}
                <Msrv @version={{@version}} />
              </span>
            {{else if @version.edition}}
              <span class='edition'>
                {{svgJar 'rust'}}
                <Edition @version={{@version}} />
              </span>
            {{/if}}

            {{#if @version.crate_size}}
              <span class='bytes'>
                {{svgJar 'weight'}}
                {{prettyBytes @version.crate_size}}
              </span>
            {{/if}}

            {{#if @version.license}}
              <span class='license'>
                {{svgJar 'license'}}
                <LicenseExpression @license={{@version.license}} />
              </span>
            {{/if}}

            {{#if @version.featureList}}
              <span class='num-features' data-test-feature-list>
                {{svgJar 'checkbox'}}
                {{@version.featureList.length}}
                {{if (eq @version.featureList.length 1) 'Feature' 'Features'}}

                <Tooltip class='tooltip'>
                  <ul class='feature-list'>
                    {{#each this.features.list as |feature|}}
                      <li>
                        {{svgJar (if feature.isDefault 'checkbox' 'checkbox-empty')}}
                        {{feature.name}}
                      </li>
                    {{/each}}
                    {{#if this.features.more}}
                      <li class='other-features'>
                        and
                        {{this.features.more}}
                        other features
                      </li>
                    {{/if}}
                  </ul>
                </Tooltip>
              </span>
            {{/if}}
          </div>
        {{/if}}
      </div>

      <PrivilegedAction @userAuthorised={{@isOwner}} class='actions'>
        <Dropdown class='dropdown' data-test-actions-menu as |dd|>
          <dd.Trigger @hideArrow={{true}} class='trigger' data-test-actions-toggle>
            {{svgJar 'ellipsis-circle' class=(scopedClass 'icon')}}
            <span class='sr-only'>Actions</span>
          </dd.Trigger>

          <dd.Menu class='menu' as |menu|>
            <menu.Item>
              <YankButton @version={{@version}} class='button-reset menu-button' />
            </menu.Item>
            <menu.Item>
              <LinkTo
                @route='crate.rebuild-docs'
                @model={{@version.num}}
                class='button-reset menu-button'
                data-test-id='btn-rebuild-docs'
              >
                Rebuild Docs
              </LinkTo>
            </menu.Item>
          </dd.Menu>
        </Dropdown>
      </PrivilegedAction>
    </div>
  </template>
}
