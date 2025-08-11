import { array, fn } from '@ember/helper';
import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import or from 'ember-truth-helpers/helpers/or';

import Placeholder from 'crates-io/components/placeholder';
import Tooltip from 'crates-io/components/tooltip';
import formatReq from 'crates-io/helpers/format-req';

export default class VersionRow extends Component {
  <template>
    <div
      data-test-dependency={{@dependency.crate_id}}
      ...attributes
      class='row {{if @dependency.optional "optional"}} {{if this.focused "focused"}} '
    >
      <span class='range-lg' data-test-range>
        {{formatReq @dependency.req}}
      </span>

      <div class='right'>
        <div class='name-and-metadata'>
          <span class='range-sm'>
            {{formatReq @dependency.req}}
          </span>

          <LinkTo
            @route='crate.range'
            @models={{array @dependency.crate_id @dependency.req}}
            class='link'
            {{on 'focusin' (fn this.setFocused true)}}
            {{on 'focusout' (fn this.setFocused false)}}
            data-test-crate-name
          >
            {{@dependency.crate_id}}
          </LinkTo>

          {{#if @dependency.optional}}
            <span class='optional-label' data-test-optional>
              optional
            </span>
          {{/if}}

          {{#if this.featuresDescription}}
            <span class='features-label' data-test-features>
              {{this.featuresDescription}}

              <Tooltip class='tooltip'>
                <ul class='feature-list'>
                  <li>
                    {{svgJar (if @dependency.default_features 'checkbox' 'checkbox-empty')}}
                    default features
                  </li>
                  {{#each @dependency.features as |feature|}}
                    <li>
                      {{svgJar 'checkbox'}}
                      {{feature}}
                    </li>
                  {{/each}}
                </ul>
              </Tooltip>
            </span>
          {{/if}}
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

  get featuresDescription() {
    let { default_features: defaultFeatures, features } = this.args.dependency;
    let numFeatures = features.length;

    if (numFeatures !== 0) {
      return defaultFeatures
        ? `${numFeatures} extra feature${numFeatures > 1 ? 's' : ''}`
        : `only ${numFeatures} feature${numFeatures > 1 ? 's' : ''}`;
    } else if (!defaultFeatures) {
      return 'no default features';
    }
  }

  loadCrateTask = task(async () => {
    let { dependency } = this.args;
    return await this.store.findRecord('crate', dependency.crate_id);
  });
}
