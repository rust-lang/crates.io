{{page-title this.crate.name}}

<CrateHeader
  @crate={{this.crate}}
  @version={{this.currentVersion}}
  @versionNum={{this.requestedVersion}}
/>

<div class='crate-info'>
  <div class="docs" data-test-docs>
    {{#if this.loadReadmeTask.isRunning}}
      <div class="readme-spinner">
        <Placeholder class="placeholder-title" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-subtitle" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-text" />
        <Placeholder class="placeholder-text" />
      </div>
    {{else if this.readme}}
      <article aria-label="Readme" data-test-readme>
        <RenderedHtml @html={{this.readme}} class="readme" />
      </article>
    {{else if this.loadReadmeTask.last.error}}
      <div class="readme-error" data-test-readme-error>
        Failed to load <code>README</code> file for {{this.crate.name}} v{{this.currentVersion.num}}

        <button
          type="button"
          class="retry-button button"
          data-test-retry-button
          {{on "click" (perform this.loadReadmeTask)}}
        >
          Retry
        </button>
      </div>
    {{else}}
      <div class="no-readme" data-test-no-readme>
        {{this.crate.name}} v{{this.currentVersion.num}} appears to have no <code>README.md</code> file
      </div>
    {{/if}}
  </div>

  <CrateSidebar
    @crate={{this.crate}}
    @version={{this.currentVersion}}
    @requestedVersion={{this.requestedVersion}}
    class="sidebar"
  />
</div>

<div class='crate-downloads'>
  <div class='stats'>
    {{#if this.downloadsContext.num}}
      <h3 data-test-crate-stats-label>
        Stats Overview for {{this.downloadsContext.num}}
        <LinkTo @route="crate" @model={{this.crate}}>(see all)</LinkTo>
      </h3>

    {{else}}
      <h3 data-test-crate-stats-label>Stats Overview</h3>
    {{/if}}
    <div class='stat'>
      <span class='num'>
        {{svg-jar "download"}}
        <span class="num__align">{{ format-num this.downloadsContext.downloads }}</span>
      </span>
      <span class="text--small">Downloads all time</span>
    </div>
    <div class='stat'>
      <span class="num">
        {{svg-jar "crate"}}
        <span class="num__align">{{ this.crate.num_versions }}</span>
      </span>
      <span class="text--small">Versions published</span>
    </div>
  </div>
  <div class='graph'>
    <h4>Downloads over the last 90 days</h4>
    <div class="toggle-stacked">
      <span class="toggle-stacked-label">Display as </span>
      <Dropdown as |dd|>
        <dd.Trigger class="trigger">
          <span class="trigger-label">
            {{#if this.stackedGraph}}
              Stacked
            {{else}}
              Unstacked
            {{/if}}
          </span>
        </dd.Trigger>
        <dd.Menu as |menu|>
          <menu.Item>
            <button
              type="button"
              class="dropdown-button"
              {{on "click" this.setStackedGraph}}
            >
              Stacked
            </button>
          </menu.Item>
          <menu.Item>
            <button
              type="button"
              class="dropdown-button"
              {{on "click" this.setUnstackedGraph}}
            >
              Unstacked
            </button>
          </menu.Item>
        </dd.Menu>
      </Dropdown>
    </div>
    <DownloadGraph @data={{this.downloads}} @stacked={{this.stackedGraph}} class="graph-data" />
  </div>
</div>