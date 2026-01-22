import { htmlSafe } from '@ember/template';

import CrateHeader from 'crates-io/components/crate-header';

<template>
  <CrateHeader @crate={{@controller.crate}} />
  {{#if @controller.advisories.length}}
    <h2 class='heading'>Advisories</h2>
    <ul class='advisories' data-test-list>
      {{#each @controller.advisories as |advisory|}}
        <li class='row'>
          <h3>
            <a href='https://rustsec.org/advisories/{{advisory.id}}.html'>{{advisory.id}}</a>:
            {{advisory.summary}}
          </h3>
          {{#if advisory.versionRanges}}
            <div class='affected-versions' data-test-affected-versions>
              <strong>Affected versions:</strong>
              {{advisory.versionRanges}}
            </div>
          {{/if}}
          {{#if advisory.aliases.length}}
            <div class='aliases' data-test-aliases>
              <strong>Aliases:</strong>
              <ul>
                {{#each advisory.aliases as |alias|}}
                  <li><a href={{@controller.aliasUrl alias}}>{{alias}}</a></li>
                {{/each}}
              </ul>
            </div>
          {{/if}}
          {{htmlSafe (@controller.convertMarkdown advisory.details)}}
        </li>
      {{/each}}
    </ul>
  {{else}}
    <div class='no-results' data-no-advisories>
      No advisories found for this crate.
    </div>
  {{/if}}
</template>
