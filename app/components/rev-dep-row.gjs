import { fn } from '@ember/helper';
import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import or from 'ember-truth-helpers/helpers/or';

import Placeholder from 'crates-io/components/placeholder';
import formatNum from 'crates-io/helpers/format-num';

export default class VersionRow extends Component {
  <template>
    <div ...attributes class='row {{if this.focused "focused"}}'>
      <div class='top'>
        <div class='left'>
          <LinkTo
            @route='crate'
            @model={{@dependency.version.crateName}}
            class='link'
            data-test-crate-name
            {{on 'focusin' (fn this.setFocused true)}}
            {{on 'focusout' (fn this.setFocused false)}}
          >
            {{@dependency.version.crateName}}
          </LinkTo>
          <span class='range'>
            depends on
            {{@dependency.req}}
          </span>
        </div>
        <div class='downloads'>
          {{svgJar 'download-arrow' class=(scopedClass 'download-icon')}}
          {{formatNum @dependency.downloads}}
        </div>
      </div>

      {{#if (or this.description this.loadCrateTask.isRunning)}}
        <div class='description' data-test-description>
          {{#if this.loadCrateTask.isRunning}}
            <Placeholder class='description-placeholder' data-test-placeholder />
          {{else}}
            {{this.description}}
          {{/if}}
        </div>
      {{/if}}
    </div>
  </template>
  @service store;

  @tracked focused = false;

  @action setFocused(value) {
    this.focused = value;
  }

  constructor() {
    super(...arguments);

    this.loadCrateTask.perform().catch(() => {
      // ignore all errors and just don't display a description if the request fails
    });
  }

  get description() {
    return this.loadCrateTask.lastSuccessful?.value?.description;
  }

  loadCrateTask = task(async () => {
    let { dependency } = this.args;
    return await this.store.findRecord('crate', dependency.version.crateName);
  });
}
