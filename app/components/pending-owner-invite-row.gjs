import { on } from '@ember/modifier';
import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';

import dateFormatDistanceToNow from 'crates-io/helpers/date-format-distance-to-now';

export default class PendingOwnerInviteRow extends Component {
  @service notifications;

  @tracked isAccepted = false;
  @tracked isDeclined = false;

  acceptInvitationTask = task(async () => {
    this.args.invite.set('accepted', true);

    try {
      await this.args.invite.save();
      this.isAccepted = true;
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in accepting invite: ${detail}`);
      } else {
        this.notifications.error('Error in accepting invite');
      }
    }
  });

  declineInvitationTask = task(async () => {
    this.args.invite.set('accepted', false);

    try {
      await this.args.invite.save();
      this.isDeclined = true;
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in declining invite: ${detail}`);
      } else {
        this.notifications.error('Error in declining invite');
      }
    }
  });

  <template>
    {{#if this.isAccepted}}
      <p data-test-accepted-message ...attributes>
        Success! You've been added as an owner of crate
        <LinkTo @route='crate' @model={{@invite.crate_name}}>{{@invite.crate_name}}</LinkTo>.
      </p>
    {{else if this.isDeclined}}
      <p data-test-declined-message ...attributes>
        Declined. You have not been added as an owner of crate
        <LinkTo @route='crate' @model={{@invite.crate_name}}>{{@invite.crate_name}}</LinkTo>.
      </p>
    {{else}}
      <div ...attributes class='row'>
        <div class='crate-column'>
          <h3>
            <LinkTo @route='crate' @model={{@invite.crate_name}} data-test-crate-link>
              {{@invite.crate_name}}
            </LinkTo>
          </h3>
        </div>
        <div>
          Invited by:
          <LinkTo @route='user' @model={{@invite.inviter.login}} data-test-inviter-link>
            {{@invite.inviter.login}}
          </LinkTo>
        </div>
        <div class='text--small' data-test-date>
          {{dateFormatDistanceToNow @invite.created_at addSuffix=true}}
        </div>
        <div>
          <button
            type='button'
            class='button button--small'
            data-test-accept-button
            {{on 'click' (perform this.acceptInvitationTask)}}
          >Accept</button>
          <button
            type='button'
            class='button button--small'
            data-test-decline-button
            {{on 'click' (perform this.declineInvitationTask)}}
          >Decline</button>
        </div>
      </div>
    {{/if}}
  </template>
}
