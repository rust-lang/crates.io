<span class="anchor" {{this.onInsertAnchor this}} />

{{#if this.visible}}
  {{#in-element this.containerElement}}
    <div class="tooltip" ...attributes {{this.attachTooltip this side=@side}}>
      {{#if (has-block)}}
        {{yield}}
      {{else}}
        {{@text}}
      {{/if}}
    </div>
  {{/in-element}}
{{/if}}
