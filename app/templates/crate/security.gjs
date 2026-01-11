import { htmlSafe } from '@ember/template';

import dateFormat from 'crates-io/helpers/date-format';
import CrateHeader from 'crates-io/components/crate-header';

<template>
  <CrateHeader @crate={{@controller.crate}} />
  {{#if @controller.advisories.length}}
    <h2 class='heading'>Advisories</h2>
    <ul class='advisories' data-test-list>
      {{#each @controller.advisories as |advisory|}}
        <li class='row'>
          {{#if advisory.withdrawn}}
            <span class='withdrawn-badge' data-test-withdrawn-badge>
              Withdrawn on
              {{dateFormat advisory.withdrawn 'MMM d, yyyy'}}
            </span>
          {{/if}}
          <h3>
            <a href='https://rustsec.org/advisories/{{advisory.id}}.html'>{{advisory.id}}</a>:
            {{advisory.summary}}
          </h3>
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
