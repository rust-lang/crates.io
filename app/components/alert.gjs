import svgJar from 'ember-svg-jar/helpers/svg-jar';
import { eq } from 'ember-truth-helpers';

<template>
  <div ...attributes class='alert' data-variant={{@variant}}>
    {{#unless @hideIcon}}
      {{#if (eq @variant 'note')}}
        {{svgJar 'alert-note'}}
      {{else if (eq @variant 'success')}}
        {{svgJar 'check-circle'}}
      {{else if (eq @variant 'tip')}}
        {{svgJar 'alert-tip'}}
      {{else if (eq @variant 'important')}}
        {{svgJar 'alert-important'}}
      {{else if (eq @variant 'warning')}}
        {{svgJar 'alert-warning'}}
      {{else if (eq @variant 'caution')}}
        {{svgJar 'alert-caution'}}
      {{/if}}
    {{/unless}}
    <div class='alert-content'>
      {{yield}}
    </div>
  </div>
</template>
