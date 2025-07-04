<form
  data-testid="crate-report-form"
  ...attributes
  class="report-form"
  {{on "submit" (prevent-default this.submit)}}
>
  <h2>Report A Crate</h2>

  <fieldset class="form-group" data-test-id="fieldset-crate">
    {{#let (unique-id) as |id|}}
      <label for={{id}} class="form-group-name">
        Crate
      </label>
      <Input
        id={{id}}
        @type="text"
        @value={{this.crate}}
        autocomplete="off"
        aria-required="true"
        aria-invalid={{if this.crateInvalid "true" "false"}}
        class="crate-input base-input"
        data-test-id="crate-input"
        {{auto-focus}}
        {{on "input" this.resetCrateValidation}}
      />
      {{#if this.crateInvalid}}
        <div class="form-group-error" data-test-id="crate-invalid">
          Please specify a crate.
        </div>
      {{/if}}
    {{/let}}
  </fieldset>

  <fieldset class="form-group" data-test-id="fieldset-reasons">
    <div class="form-group-name">Reasons</div>
    <ul role="list" class="reasons-list scopes-list {{if this.reasonsInvalid "invalid"}}">
      {{#each this.reasons as |option|}}
        <li>
          <label>
            <Input
              @type="checkbox"
              @checked={{this.isReasonSelected option.reason}}
              name={{ option.reason }}
              data-test-id="{{ option.reason }}-checkbox"
              {{on "change" (fn this.toggleReason option.reason)}}
            />
            {{option.description}}
          </label>
        </li>
      {{/each}}
    </ul>
    {{#if this.reasonsInvalid}}
      <div class="form-group-error" data-test-id="reasons-invalid">
        Please choose reasons to report.
      </div>
    {{/if}}
  </fieldset>

  <fieldset class="form-group" data-test-id="fieldset-detail">
    {{#let (unique-id) as |id|}}
      <label for={{id}} class="form-group-name">Detail</label>
      <Textarea
        id={{id}}
        @value={{this.detail}}
        class="detail {{if this.detailInvalid "invalid"}}"
        aria-required={{if this.detailInvalid "true" "false" }}
        aria-invalid={{if this.detailInvalid "true" "false"}}
        rows="5"
        data-test-id="detail-input"
        {{on "input" this.resetDetailValidation}}
      />
      {{#if this.detailInvalid}}
        <div class="form-group-error" data-test-id="detail-invalid">
          Please provide some detail.
        </div>
      {{/if}}
    {{/let}}
  </fieldset>

  <div class="buttons">
    <button
      type="submit"
      class="report-button button button--small"
      data-test-id="report-button"
    >
      Report to <strong>help@crates.io</strong>
    </button>
  </div>
</form>