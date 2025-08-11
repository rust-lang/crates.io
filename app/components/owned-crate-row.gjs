import { action } from '@ember/object';
import Component from '@glimmer/component';

export default class OwnedCrateRow extends Component {
  @action setEmailNotifications(event) {
    let { checked } = event.target;
    this.args.ownedCrate.set('email_notifications', checked);
  }
}

<label
  data-test-owned-crate={{@ownedCrate.name}}
  ...attributes
  class="label {{if @ownedCrate.email_notifications "checked"}}"
>
  <span class="name">
    {{@ownedCrate.name}}
  </span>

  <div aria-hidden="true" class="checkbox">
    {{#if @ownedCrate.email_notifications}}
      {{svg-jar "check-mark"}}
    {{/if}}
  </div>

  <input
    type="checkbox"
    checked={{@ownedCrate.email_notifications}}
    class="sr-only"
    {{on "change" this.setEmailNotifications}}
  />
</label>