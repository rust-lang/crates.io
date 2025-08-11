<div
  data-test-dependency={{@dependency.crate_id}}
  ...attributes
  class="
    row
    {{if @dependency.optional "optional"}}
    {{if this.focused "focused"}}
  "
>
  <span class="range-lg" data-test-range>
    {{format-req @dependency.req}}
  </span>

  <div class="right">
    <div class="name-and-metadata">
      <span class="range-sm">
        {{format-req @dependency.req}}
      </span>

      <LinkTo
        @route="crate.range"
        @models={{array @dependency.crate_id @dependency.req}}
        class="link"
        {{on "focusin" (fn this.setFocused true)}}
        {{on "focusout" (fn this.setFocused false)}}
        data-test-crate-name
      >
        {{@dependency.crate_id}}
      </LinkTo>

      {{#if @dependency.optional}}
        <span class="optional-label" data-test-optional>
          optional
        </span>
      {{/if}}

      {{#if this.featuresDescription}}
        <span class="features-label" data-test-features>
          {{this.featuresDescription}}

          <Tooltip class="tooltip">
            <ul class="feature-list">
              <li>
                {{svg-jar (if @dependency.default_features "checkbox" "checkbox-empty")}} default features
              </li>
              {{#each @dependency.features as |feature|}}
                <li>
                  {{svg-jar "checkbox"}} {{feature}}
                </li>
              {{/each}}
            </ul>
          </Tooltip>
        </span>
      {{/if}}
    </div>

    {{#if (or this.description this.loadCrateTask.isRunning)}}
      <div class="description" data-test-description>
        {{#if this.loadCrateTask.isRunning}}
          <Placeholder class="description-placeholder" data-test-placeholder />
        {{else}}
          {{this.description}}
        {{/if}}
      </div>
    {{/if}}
  </div>
</div>