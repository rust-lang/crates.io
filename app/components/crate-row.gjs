<div data-test-crate-row ...attributes class="crate-row">
  <div class="description-box">
    <div class="crate-spec">
      {{#let (link "crate" @crate.id) as |l|}}
        <a href={{l.url}} class="name" data-test-crate-link {{on "click" l.transitionTo}}>
          {{@crate.name}}
        </a>
      {{/let}}
      {{#if (and @crate.default_version (not @crate.yanked))}}
        <span class="version" data-test-version>v{{@crate.default_version}}</span>
        <CopyButton
          @copyText='{{@crate.name}} = "{{@crate.default_version}}"'
          title="Copy Cargo.toml snippet to clipboard"
          class="copy-button button-reset"
          data-test-copy-toml-button
        >
          {{svg-jar "copy" alt="Copy Cargo.toml snippet to clipboard"}}
        </CopyButton>
      {{/if}}
    </div>
    <div class="description text--small" data-test-description>
      {{ truncate-text @crate.description }}
    </div>
  </div>
  <div class='stats'>
    <div class='downloads' data-test-downloads>
      {{svg-jar "download" class=(scoped-class "download-icon")}}
      <span>
        <span>
          All-Time:
          <Tooltip @text="Total number of downloads"/>
        </span>
        {{ format-num @crate.downloads }}
      </span>
    </div>
    <div class="recent-downloads" data-test-recent-downloads>
      {{svg-jar "download" class=(scoped-class "download-icon")}}
      <span>
        <span>
          Recent:
          <Tooltip @text="Downloads in the last 90 days"/>
        </span>
        {{ format-num @crate.recent_downloads }}
      </span>
    </div>
    <div class="updated-at">
      {{svg-jar "latest-updates" height="32" width="32"}}
      <span>
        <span>
          Updated:
          <Tooltip @text="The last time the crate was updated" />
        </span>
        <time datetime="{{date-format-iso @crate.updated_at}}" data-test-updated-at>
          {{date-format-distance-to-now @crate.updated_at addSuffix=true}}
          <Tooltip @text={{ @crate.updated_at }}/>
        </time>
      </span>
    </div>
  </div>
  <ul class="quick-links">
    {{#if @crate.homepage}}
      <li><a href="{{@crate.homepage}}">Homepage</a></li>
    {{/if}}
    {{#if @crate.documentation}}
      <li><a href="{{@crate.documentation}}">Documentation</a></li>
    {{/if}}
    {{#if @crate.repository}}
      <li><a href="{{@crate.repository}}">Repository</a></li>
    {{/if}}
  </ul>

</div>