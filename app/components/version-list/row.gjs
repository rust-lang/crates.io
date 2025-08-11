import { action } from '@ember/object';
import { service } from '@ember/service';
import { htmlSafe } from '@ember/template';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import styles from './row.css';

export default class VersionRow extends Component {
  @service session;

  @tracked focused = false;

  get releaseTrackTitle() {
    let { version } = this.args;
    if (version.yanked) {
      return htmlSafe(`This version was <span class="${styles['rt-yanked']}">yanked</span>`);
    }
    if (version.invalidSemver) {
      return `Failed to parse version ${version.num}`;
    }

    let { releaseTrack } = version;

    let modifiers = [];
    if (version.isPrerelease) {
      modifiers.push('prerelease');
    }
    if (version.isHighestOfReleaseTrack) {
      modifiers.push('latest');
    }

    let title = `Release Track: ${releaseTrack}`;
    if (modifiers.length !== 0) {
      let formattedModifiers = modifiers
        .map(modifier => {
          let klass = styles[`rt-${modifier}`];
          return klass ? `<span class='${klass}'>${modifier}</span>` : modifier;
        })
        .join(', ');

      title += ` (${formattedModifiers})`;
    }
    return htmlSafe(title);
  }

  get isOwner() {
    let userId = this.session.currentUser?.id;
    return this.args.version.crate.hasOwnerUser(userId);
  }

  get features() {
    let features = this.args.version.featureList;
    let list = features.slice(0, 15);
    let more = features.length - list.length;
    return { list, more };
  }

  @action setFocused(value) {
    this.focused = value;
  }
}

<div
  ...attributes
  class="
    row
    {{if @version.isHighestOfReleaseTrack "latest"}}
    {{if @version.yanked "yanked"}}
    {{if @version.isPrerelease "prerelease"}}
    {{if this.focused "focused"}}
  "
>
  <div class="version">
    <div class="release-track" data-test-release-track>
      {{#if @version.yanked}}
        {{svg-jar "trash"}}
      {{else if @version.invalidSemver}}
        ?
      {{else}}
        {{@version.releaseTrack}}
      {{/if}}

      <Tooltip @side="right" class="rt-tooltip" data-test-release-track-title>
        {{this.releaseTrackTitle}}
      </Tooltip>
    </div>

    <LinkTo
      @route="crate.version"
      @model={{@version.num}}
      class="num-link"
      {{on "focusin" (fn this.setFocused true)}}
      {{on "focusout" (fn this.setFocused false)}}
      data-test-release-track-link
    >
      {{@version.num}}
    </LinkTo>
  </div>

  <div class="metadata">
    <div class="metadata-row">
      {{#if @version.published_by}}
        <span class="publisher">
          by
          <LinkTo @route="user" @model={{@version.published_by.login}}>
            <UserAvatar @user={{@version.published_by}} class="avatar" />
            {{or @version.published_by.name @version.published_by.login}}
          </LinkTo>
        </span>
      {{else if @version.trustpubPublisher}}
        <span local-class="publisher trustpub">
          via
          {{#if @version.trustpubUrl}}
            <a href={{@version.trustpubUrl}} target="_blank" rel="nofollow noopener noreferrer">
              {{#if (eq @version.trustpub_data.provider "github")}}
                {{svg-jar "github"}}
              {{/if}}
              {{@version.trustpubPublisher}}
            </a>
          {{else}}
            {{#if (eq @version.trustpub_data.provider "github")}}
              {{svg-jar "github"}}
            {{/if}}
            {{@version.trustpubPublisher}}
          {{/if}}
        </span>
      {{/if}}

      <time
        datetime={{date-format-iso @version.created_at}}
        class="date {{if @version.isNew "new"}}"
      >
        {{svg-jar "calendar"}}
        {{date-format-distance-to-now @version.created_at addSuffix=true}}

        <Tooltip class="tooltip">
          {{date-format @version.created_at 'PPP'}}
          {{#if @version.isNew}}
            (<span class="new">new</span>)
          {{/if}}
        </Tooltip>
      </time>
    </div>

    {{#if (or @version.crate_size @version.license @version.featureList)}}
      <div class="metadata-row">
        {{#if @version.rust_version}}
          <span class="msrv">
            {{svg-jar "rust"}}
            <Msrv @version={{@version}} />
          </span>
        {{else if @version.edition}}
          <span class="edition">
            {{svg-jar "rust"}}
            <Edition @version={{@version}} />
          </span>
        {{/if}}

        {{#if @version.crate_size}}
          <span class="bytes">
            {{svg-jar "weight"}}
            {{pretty-bytes @version.crate_size}}
          </span>
        {{/if}}

        {{#if @version.license}}
          <span class="license">
            {{svg-jar "license"}}
            <LicenseExpression @license={{@version.license}} />
          </span>
        {{/if}}

        {{#if @version.featureList}}
          <span class="num-features" data-test-feature-list>
            {{svg-jar "checkbox"}}
            {{@version.featureList.length}} {{if (eq @version.featureList.length 1) "Feature" "Features"}}

            <Tooltip class="tooltip">
              <ul class="feature-list">
                {{#each this.features.list as |feature|}}
                  <li>
                    {{svg-jar (if feature.isDefault "checkbox" "checkbox-empty")}}
                    {{feature.name}}
                  </li>
                {{/each}}
                {{#if this.features.more}}
                  <li class="other-features">
                    and {{this.features.more}} other features
                  </li>
                {{/if}}
              </ul>
            </Tooltip>
          </span>
        {{/if}}
      </div>
    {{/if}}
  </div>

  <PrivilegedAction @userAuthorised={{this.isOwner}} class="actions">
    <Dropdown class="dropdown" data-test-actions-menu as |dd|>
      <dd.Trigger @hideArrow={{true}} class="trigger" data-test-actions-toggle>
        {{svg-jar "ellipsis-circle" class=(scoped-class "icon")}}
        <span class="sr-only">Actions</span>
      </dd.Trigger>

      <dd.Menu class="menu" as |menu|>
        <menu.Item>
          <YankButton @version={{@version}} class="button-reset menu-button" />
        </menu.Item>
        <menu.Item>
          <LinkTo
            @route="crate.rebuild-docs"
            @model={{@version.num}}
            class="button-reset menu-button"
            data-test-id="btn-rebuild-docs"
          >
            Rebuild Docs
          </LinkTo>
        </menu.Item>
      </dd.Menu>
    </Dropdown>
  </PrivilegedAction>
</div>