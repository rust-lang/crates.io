{{#if @binNames}}
  {{#if (is-clipboard-supported)}}
    <CopyButton
      @copyText={{this.cargoInstallCommand}}
      title="Copy command to clipboard"
      class="copy-button"
    >
      <span class="selectable">{{this.cargoInstallCommand}}</span>
      {{svg-jar "copy" aria-hidden="true" class=(scoped-class "copy-icon")}}
    </CopyButton>
  {{else}}
    <code class="copy-fallback">
      {{this.cargoInstallCommand}}
    </code>
  {{/if}}

  <p class="copy-help">
    {{#if (eq @binNames.length 1)}}
      Running the above command will globally install the
      <span class="bin-name">{{get @binNames 0}}</span>
      binary.
    {{else if (eq @binNames.length 2)}}
      Running the above command will globally install the
      <span class="bin-name">{{get @binNames 0}}</span>
      and
      <span class="bin-name">{{get @binNames 1}}</span>
      binaries.
    {{else}}
      Running the above command will globally install these binaries:
      {{#each @binNames as |binName index|~}}
        {{~#if (eq index 0)~}}
          <span class="bin-name">{{binName}}</span>
        {{~else if (eq index (sum @binNames.length -1))}}
          and <span class="bin-name">{{binName}}</span>
        {{~else~}}
          , <span class="bin-name">{{binName}}</span>
        {{~/if}}
      {{~/each}}
    {{/if}}
  </p>

{{/if}}

{{#if (and @hasLib @binNames)}}
  <h3>Install as library</h3>
{{/if}}

{{#if @hasLib}}
  <p class="copy-help">Run the following Cargo command in your project directory:</p>

  {{#if (is-clipboard-supported)}}
    <CopyButton
      @copyText={{this.cargoAddCommand}}
      title="Copy command to clipboard"
      class="copy-button"
    >
      <span class="selectable">{{this.cargoAddCommand}}</span>
      {{svg-jar "copy" aria-hidden="true" class=(scoped-class "copy-icon")}}
    </CopyButton>
  {{else}}
    <code class="copy-fallback">
      {{this.cargoAddCommand}}
    </code>
  {{/if}}

  <p class="copy-help">Or add the following line to your Cargo.toml:</p>

  {{#if (is-clipboard-supported)}}
    <CopyButton
      @copyText={{this.tomlSnippet}}
      title="Copy Cargo.toml snippet to clipboard"
      class="copy-button"
    >
      <span class="selectable">{{this.tomlSnippet}}</span>
      {{svg-jar "copy" aria-hidden="true" class=(scoped-class "copy-icon")}}
    </CopyButton>
  {{else}}
    <code class="copy-fallback">
      {{this.tomlSnippet}}
    </code>
  {{/if}}
{{/if}}