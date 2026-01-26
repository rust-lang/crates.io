import { htmlSafe } from '@ember/template';

import CrateHeader from 'crates-io/components/crate-header';

function aliasUrl(alias) {
  if (alias.startsWith('CVE-')) {
    return `https://nvd.nist.gov/vuln/detail/${alias}`;
  } else if (alias.startsWith('GHSA-')) {
    return `https://github.com/advisories/${alias}`;
  }
  return null;
}

function cvssUrl(cvss) {
  // Extract version from CVSS string (e.g., "CVSS:3.1/..." -> "3.1")
  let match = cvss.match(/^CVSS:(\d+\.\d+)\//);
  if (match) {
    return `https://www.first.org/cvss/calculator/${match[1]}#${cvss}`;
  }
  return null;
}

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
                  <li><a href={{aliasUrl alias}}>{{alias}}</a></li>
                {{/each}}
              </ul>
            </div>
          {{/if}}
          {{#if advisory.cvss}}
            <div class='cvss' data-test-cvss>
              <strong>CVSS:</strong>
              <a href={{cvssUrl advisory.cvss}}>{{advisory.cvss}}</a>
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
