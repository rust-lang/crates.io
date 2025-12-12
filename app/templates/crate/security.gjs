import { htmlSafe } from '@ember/template';

import CrateHeader from 'crates-io/components/crate-header';

<template>
  <CrateHeader @crate={{@controller.crate}} />
  {{#if @controller.model}}
    <h2 class='heading'>Advisories</h2>
    <ul class='advisories' data-test-list>
      {{#each @controller.advisories as |advisory|}}
        <li class='row'>
          <h3>
            <a href='https://rustsec.org/advisories/{{advisory.id}}.html'>{{advisory.id}}</a>:
            {{advisory.summary}}
          </h3>
          <p>{{htmlSafe (@controller.convertMarkdown advisory.details)}}</p>
        </li>
      {{/each}}
    </ul>
  {{else}}
    <div class='no-results'>
      No advisories found for this crate.
    </div>
  {{/if}}
</template>
