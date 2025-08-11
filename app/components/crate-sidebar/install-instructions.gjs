import { get } from '@ember/helper';
import Component from '@glimmer/component';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import and from 'ember-truth-helpers/helpers/and';
import eq from 'ember-truth-helpers/helpers/eq';

import CopyButton from 'crates-io/components/copy-button';
import isClipboardSupported from 'crates-io/helpers/is-clipboard-supported';
import sum from 'crates-io/helpers/sum';

export default class InstallInstructions extends Component {
  <template>
    {{#if @binNames}}
      {{#if (isClipboardSupported)}}
        <CopyButton @copyText={{this.cargoInstallCommand}} title='Copy command to clipboard' class='copy-button'>
          <span class='selectable'>{{this.cargoInstallCommand}}</span>
          {{svgJar 'copy' aria-hidden='true' class=(scopedClass 'copy-icon')}}
        </CopyButton>
      {{else}}
        <code class='copy-fallback'>
          {{this.cargoInstallCommand}}
        </code>
      {{/if}}

      <p class='copy-help'>
        {{#if (eq @binNames.length 1)}}
          Running the above command will globally install the
          <span class='bin-name'>{{get @binNames 0}}</span>
          binary.
        {{else if (eq @binNames.length 2)}}
          Running the above command will globally install the
          <span class='bin-name'>{{get @binNames 0}}</span>
          and
          <span class='bin-name'>{{get @binNames 1}}</span>
          binaries.
        {{else}}
          Running the above command will globally install these binaries:
          {{#each @binNames as |binName index|~}}
            {{~#if (eq index 0)~}}
              <span class='bin-name'>{{binName}}</span>
            {{~else if (eq index (sum @binNames.length -1))~}}
              and
              <span class='bin-name'>{{binName}}</span>
            {{~else~}}
              ,
              <span class='bin-name'>{{binName}}</span>
            {{~/if}}
          {{~/each}}
        {{/if}}
      </p>

    {{/if}}

    {{#if (and @hasLib @binNames)}}
      <h3>Install as library</h3>
    {{/if}}

    {{#if @hasLib}}
      <p class='copy-help'>Run the following Cargo command in your project directory:</p>

      {{#if (isClipboardSupported)}}
        <CopyButton @copyText={{this.cargoAddCommand}} title='Copy command to clipboard' class='copy-button'>
          <span class='selectable'>{{this.cargoAddCommand}}</span>
          {{svgJar 'copy' aria-hidden='true' class=(scopedClass 'copy-icon')}}
        </CopyButton>
      {{else}}
        <code class='copy-fallback'>
          {{this.cargoAddCommand}}
        </code>
      {{/if}}

      <p class='copy-help'>Or add the following line to your Cargo.toml:</p>

      {{#if (isClipboardSupported)}}
        <CopyButton @copyText={{this.tomlSnippet}} title='Copy Cargo.toml snippet to clipboard' class='copy-button'>
          <span class='selectable'>{{this.tomlSnippet}}</span>
          {{svgJar 'copy' aria-hidden='true' class=(scopedClass 'copy-icon')}}
        </CopyButton>
      {{else}}
        <code class='copy-fallback'>
          {{this.tomlSnippet}}
        </code>
      {{/if}}
    {{/if}}
  </template>
  get cargoInstallCommand() {
    return this.args.exactVersion
      ? `cargo install ${this.args.crate}@${this.args.version}`
      : `cargo install ${this.args.crate}`;
  }

  get cargoAddCommand() {
    return this.args.exactVersion
      ? `cargo add ${this.args.crate}@=${this.args.version}`
      : `cargo add ${this.args.crate}`;
  }

  get tomlSnippet() {
    let version = this.args.version.split('+')[0];
    let exact = this.args.exactVersion ? '=' : '';
    return `${this.args.crate} = "${exact}${version}"`;
  }
}
