import { Input } from '@ember/component';
import { fn } from '@ember/helper';
import { on } from '@ember/modifier';
import { action } from '@ember/object';
import { service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';
import preventDefault from 'ember-event-helpers/helpers/prevent-default';
import and from 'ember-truth-helpers/helpers/and';
import not from 'ember-truth-helpers/helpers/not';

export default class EmailInput extends Component {
  <template>
    <div ...attributes>
      {{#unless @user.email}}
        <div class='friendly-message' data-test-no-email>
          <p>
            Please add your email address. We will only use it to contact you about your account. We promise we'll never
            share it!
          </p>
        </div>
      {{/unless}}

      {{#if this.isEditing}}
        <div class='row'>
          <div class='label'>
            <label for='email-input'>Email</label>
          </div>
          <form class='email-form' {{on 'submit' (preventDefault (perform this.saveEmailTask))}}>
            <Input
              @type='email'
              @value={{this.value}}
              id='email-input'
              placeholder='Email'
              class='input'
              data-test-input
            />

            <div class='actions'>
              <button
                type='submit'
                class='save-button button button--small'
                disabled={{not this.value}}
                data-test-save-button
              >
                Save
              </button>

              <button
                type='button'
                class='button button--small'
                data-test-cancel-button
                {{on 'click' (fn (mut this.isEditing) false)}}
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      {{else}}
        <div class='row'>
          <div class='label'>
            <dt>Email</dt>
          </div>
          <div class='email-column' data-test-email-address>
            <dd>
              {{@user.email}}
              {{#if @user.email_verified}}
                <span class='verified' data-test-verified>Verified!</span>
              {{/if}}
            </dd>
          </div>
          <div class='actions'>
            <button type='button' class='button button--small' data-test-edit-button {{on 'click' this.editEmail}}>
              Edit
            </button>
          </div>
        </div>
        {{#if (and @user.email (not @user.email_verified))}}
          <div class='row'>
            <div class='label'>
              {{#if @user.email_verification_sent}}
                <p data-test-verification-sent>We have sent a verification email to your address.</p>
              {{/if}}
              <p data-test-not-verified>Your email has not yet been verified.</p>
            </div>
            <div class='actions'>
              <button
                type='button'
                class='button button--small'
                disabled={{this.disableResend}}
                data-test-resend-button
                {{on 'click' (perform this.resendEmailTask)}}
              >
                {{#if this.disableResend}}
                  Sent!
                {{else if @user.email_verification_sent}}
                  Resend
                {{else}}
                  Send verification email
                {{/if}}
              </button>
            </div>
          </div>
        {{/if}}
      {{/if}}

    </div>
  </template>
  @service notifications;

  @tracked value;
  @tracked isEditing = false;
  @tracked disableResend = false;

  resendEmailTask = task(async () => {
    try {
      await this.args.user.resendVerificationEmail();
      this.disableResend = true;
    } catch (error) {
      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        this.notifications.error(`Error in resending message: ${detail}`);
      } else {
        this.notifications.error('Unknown error in resending message');
      }
    }
  });

  @action
  editEmail() {
    this.value = this.args.user.email;
    this.isEditing = true;
  }

  saveEmailTask = task(async () => {
    let userEmail = this.value;
    let user = this.args.user;

    try {
      await user.changeEmail(userEmail);

      this.isEditing = false;
      this.disableResend = false;
    } catch (error) {
      let detail = error.errors?.[0]?.detail;

      let msg =
        detail && !detail.startsWith('{')
          ? `An error occurred while saving this email, ${detail}`
          : 'An unknown error occurred while saving this email.';

      this.notifications.error(`Error in saving email: ${msg}`);
    }
  });
}
