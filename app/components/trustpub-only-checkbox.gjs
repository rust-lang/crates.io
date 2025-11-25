import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import LoadingSpinner from 'crates-io/components/loading-spinner';

export default class TrustpubOnlyCheckbox extends Component {
  @service notifications;

  @action async toggle(event) {
    let { checked } = event.target;
    try {
      await this.args.crate.setTrustpubOnlyTask.perform(checked);
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(detail);
      } else {
        this.notifications.error('Failed to update trusted publishing setting');
      }
    }
  }

  <template>
    <label class='trustpub-only-checkbox' data-test-trustpub-only-checkbox ...attributes>
      <div class='checkbox'>
        {{#if @crate.setTrustpubOnlyTask.isRunning}}
          <LoadingSpinner data-test-spinner />
        {{else}}
          <input type='checkbox' checked={{@crate.trustpub_only}} data-test-checkbox {{on 'change' this.toggle}} />
        {{/if}}
      </div>
      <div class='label'>Require trusted publishing for all new versions</div>
      <div class='note'>
        When enabled, new versions can only be published through configured trusted publishers. Publishing with API
        tokens will be rejected.
      </div>
    </label>
  </template>
}
