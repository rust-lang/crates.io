import Component from '@glimmer/component';

import { eq } from 'ember-truth-helpers';

export default class PatternDescription extends Component {
  get prefix() {
    if (this.args.pattern.endsWith('*')) {
      return this.args.pattern.slice(0, -1);
    }
  }

  <template>
    {{#if (eq @pattern '*')}}
      Matches all crates on crates.io
    {{else if this.prefix}}
      Matches all crates starting with
      <strong>{{this.prefix}}</strong>
    {{else}}
      Matches only the
      <strong>{{@pattern}}</strong>
      crate
    {{/if}}
  </template>
}
