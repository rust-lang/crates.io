{{page-title 'Rebuild Documentation'}}

<div class="content">
  <h1 data-test-title>Rebuild Documentation</h1>

  <div class="crate-info">
    <h2>Crate Information</h2>
    <div class="info-row">
      <strong>Crate:</strong> <span data-test-crate-name>{{@model.crate.name}}</span>
    </div>
    <div class="info-row">
      <strong>Version:</strong> <span data-test-version-num>{{@model.version.num}}</span>
    </div>
  </div>

  <div class="description">
    <p>
      This will trigger a rebuild of the documentation for
      <a href="https://docs.rs/{{@model.crate.name}}/{{@model.version.num}}" target="_blank" rel="noopener noreferrer">
        <strong>{{@model.crate.name}} {{@model.version.num}}</strong>
      </a>
      on docs.rs.
    </p>
    <p>
      The rebuild process may take several minutes to complete. You can monitor the build progress at the <a href="https://docs.rs/releases/queue" target="_blank" rel="noopener noreferrer">docs.rs build queue</a>.
    </p>
  </div>

  <div class="actions">
    <button
      type="button"
      class="button button--yellow"
      disabled={{this.rebuildTask.isRunning}}
      data-test-confirm-rebuild-button
      {{on "click" (perform this.rebuildTask)}}
    >
      {{#if this.rebuildTask.isRunning}}
        Requesting Rebuild...
      {{else}}
        Confirm Rebuild
      {{/if}}
    </button>
    <LinkTo @route="crate.versions" @model={{@model.crate.name}} class="button button--tan" data-test-cancel-button>
      Cancel
    </LinkTo>
  </div>
</div>