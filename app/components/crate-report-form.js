import { action } from '@ember/object';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

export default class CrateReportForm extends Component {
  @tracked selectedReasons = [];
  @tracked detail = '';
  @tracked reasonsInvalid;
  @tracked detailInvalid;

  reasons = [
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

  constructor() {
    super(...arguments);
    this.reset();
  }

  reset() {
    this.reasonsInvalid = false;
    this.detailInvalid = false;
  }

  validate() {
    this.reasonsInvalid = this.selectedReasons.length === 0;
    this.detailInvalid = this.selectedReasons.includes('other') && !this.detail?.trim();
    return !this.reasonsInvalid && !this.detailInvalid;
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

  @action updateDetail(event) {
    let { value } = event?.target ?? {};
    this.detail = value ?? '';
  }

  @action cancel() {
    this.args.close?.();
  }

  @action submit() {
    if (!this.validate()) {
      return;
    }

    let mailto = this.composeMail();
    window.open(mailto, '_self');
    this.args.close?.();
  }

  composeMail() {
    let name = this.args.crate ?? '';
    let reasons = this.reasons
      .map(({ reason, description }) => {
        let selected = this.isReasonSelected(reason);
        return `${selected ? '- [x]' : '- [ ]'} ${description}`;
      })
      .join('\n');
    let body = `I'm reporting the https://crates.io/crates/${name} crate because:

${reasons}

Additional details:

${this.detail}
`;
    let subject = `The "${name}" crate`;
    let address = 'help@crates.io';
    let mailto = `mailto:${address}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
    return mailto;
  }
}
