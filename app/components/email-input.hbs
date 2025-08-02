<div ...attributes>
  {{#unless @user.email}}
    <div class="friendly-message" data-test-no-email>
      <p>
        Please add your email address. We will only use
        it to contact you about your account. We promise we'll never share it!
      </p>
    </div>
  {{/unless}}

  {{#if this.isEditing }}
    <div class="row">
      <div class="label">
        <label for="email-input">Email</label>
      </div>
      <form class="email-form" {{on "submit" (prevent-default (perform this.saveEmailTask))}}>
        <Input
          @type="email"
          @value={{this.value}}
          id="email-input"
          placeholder="Email"
          class="input"
          data-test-input
        />

        <div class="actions">
          <button
            type='submit'
            class="save-button button button--small"
            disabled={{not this.value}}
            data-test-save-button
          >
            Save
          </button>

          <button
            type="button"
            class="button button--small"
            data-test-cancel-button
            {{on "click" (fn (mut this.isEditing) false)}}
          >
            Cancel
          </button>
        </div>
      </form>
    </div>
  {{else}}
    <div class="row">
      <div class="label">
        <dt>Email</dt>
      </div>
      <div class="email-column" data-test-email-address>
        <dd>
          {{ @user.email }}
          {{#if @user.email_verified}}
            <span class="verified" data-test-verified>Verified!</span>
          {{/if}}
        </dd>
      </div>
      <div class="actions">
        <button
          type="button"
          class="button button--small"
          data-test-edit-button
          {{on "click" this.editEmail}}
        >
          Edit
        </button>
      </div>
    </div>
    {{#if (and @user.email (not @user.email_verified))}}
      <div class="row">
        <div class="label">
          {{#if @user.email_verification_sent}}
            <p data-test-verification-sent>We have sent a verification email to your address.</p>
          {{/if}}
          <p data-test-not-verified>Your email has not yet been verified.</p>
        </div>
        <div class="actions">
          <button
            type="button"
            class="button button--small"
            disabled={{this.disableResend}}
            data-test-resend-button
            {{on "click" (perform this.resendEmailTask)}}
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