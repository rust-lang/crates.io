import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

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
}
