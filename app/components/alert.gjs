import svgJar from 'ember-svg-jar/helpers/svg-jar';
import { eq } from 'ember-truth-helpers';

<template>
  <div ...attributes class='alert' data-variant={{@variant}}>
    {{#unless @hideIcon}}
      {{#if (eq @variant 'warning')}}
        {{svgJar 'alert-warning'}}
      {{/if}}
    {{/unless}}
    <div class='alert-content'>
      {{yield}}
    </div>
  </div>
</template>
