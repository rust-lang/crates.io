import { Input, Textarea } from '@ember/component';
import { fn, uniqueId } from '@ember/helper';
import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import autoFocus from '@zestia/ember-auto-focus/modifiers/auto-focus';
import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import window from 'ember-window-mock';

const REASONS = [
  {
    reason: 'spam',
    description: 'it contains spam',
  },
  {
    reason: 'name-squatting',
    description: 'it is name-squatting (reserving a crate name without content)',
  },
  {
    reason: 'abuse',
    description: 'it is abusive or otherwise harmful',
  },
  {
    reason: 'security',
    description: 'it contains a vulnerability (please try to contact the crate author first)',
  },
  {
    reason: 'other',
    description: 'it is violating the usage policy in some other way (please specify below)',
  },
];

export default class CrateReportForm extends Component {
  @service store;

  @tracked crate = '';
  @tracked selectedReasons = [];
  @tracked detail = '';
  @tracked crateInvalid = false;
  @tracked reasonsInvalid = false;
  @tracked detailInvalid = false;

  reasons = REASONS;

  constructor() {
    super(...arguments);
    this.crate = this.args.crate;
  }

  validate() {
    this.crateInvalid = !this.crate || !this.crate.trim();
    this.reasonsInvalid = this.selectedReasons.length === 0;
    this.detailInvalid = this.selectedReasons.includes('other') && !this.detail?.trim();
    return !this.crateInvalid && !this.reasonsInvalid && !this.detailInvalid;
  }

  @action resetCrateValidation() {
    this.crateInvalid = false;
  }

  @action resetDetailValidation() {
    this.detailInvalid = false;
  }

  @action isReasonSelected(reason) {
    return this.selectedReasons.includes(reason);
  }

  @action toggleReason(reason) {
    this.selectedReasons = this.selectedReasons.includes(reason)
      ? this.selectedReasons.filter(it => it !== reason)
      : [...this.selectedReasons, reason];
    this.reasonsInvalid = false;
  }

  @action
  submit() {
    if (!this.validate()) {
      return;
    }

    let mailto = this.composeMail();
    window.open(mailto, '_self');
  }

  composeMail() {
    let crate = this.crate;
    let reasons = this.reasons
      .map(({ reason, description }) => {
        let selected = this.isReasonSelected(reason);
        return `${selected ? '- [x]' : '- [ ]'} ${description}`;
      })
      .join('\n');
    let body = `I'm reporting the https://crates.io/crates/${crate} crate because:

${reasons}

Additional details:

${this.detail}
`;
    let subject = `The "${crate}" crate`;
    let address = 'help@crates.io';
    let mailto = `mailto:${address}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
    return mailto;
  }

  <template>
    <form
      data-testid='crate-report-form'
      ...attributes
      class='report-form'
      {{on 'submit' (preventDefault this.submit)}}
    >
      <h2>Report A Crate</h2>

      <fieldset class='form-group' data-test-id='fieldset-crate'>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>
            Crate
          </label>
          <Input
            id={{id}}
            @type='text'
            @value={{this.crate}}
            autocomplete='off'
            aria-required='true'
            aria-invalid={{if this.crateInvalid 'true' 'false'}}
            class='crate-input base-input'
            data-test-id='crate-input'
            {{autoFocus}}
            {{on 'input' this.resetCrateValidation}}
          />
          {{#if this.crateInvalid}}
            <div class='form-group-error' data-test-id='crate-invalid'>
              Please specify a crate.
            </div>
          {{/if}}
        {{/let}}
      </fieldset>

      <fieldset class='form-group' data-test-id='fieldset-reasons'>
        <div class='form-group-name'>Reasons</div>
        <ul role='list' class='reasons-list scopes-list {{if this.reasonsInvalid "invalid"}}'>
          {{#each this.reasons as |option|}}
            <li>
              <label>
                <Input
                  @type='checkbox'
                  @checked={{this.isReasonSelected option.reason}}
                  name={{option.reason}}
                  data-test-id='{{option.reason}}-checkbox'
                  {{on 'change' (fn this.toggleReason option.reason)}}
                />
                {{option.description}}
              </label>
            </li>
          {{/each}}
        </ul>
        {{#if this.reasonsInvalid}}
          <div class='form-group-error' data-test-id='reasons-invalid'>
            Please choose reasons to report.
          </div>
        {{/if}}
      </fieldset>

      <fieldset class='form-group' data-test-id='fieldset-detail'>
        {{#let (uniqueId) as |id|}}
          <label for={{id}} class='form-group-name'>Detail</label>
          <Textarea
            id={{id}}
            @value={{this.detail}}
            class='detail {{if this.detailInvalid "invalid"}}'
            aria-required={{if this.detailInvalid 'true' 'false'}}
            aria-invalid={{if this.detailInvalid 'true' 'false'}}
            rows='5'
            data-test-id='detail-input'
            {{on 'input' this.resetDetailValidation}}
          />
          {{#if this.detailInvalid}}
            <div class='form-group-error' data-test-id='detail-invalid'>
              Please provide some detail.
            </div>
          {{/if}}
        {{/let}}
      </fieldset>

      <div class='buttons'>
        <button type='submit' class='report-button button button--small' data-test-id='report-button'>
          Report to
          <strong>help@crates.io</strong>
        </button>
      </div>
    </form>
  </template>
}
