import { on } from '@ember/modifier';

import perform from 'ember-concurrency/helpers/perform';
import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';
import or from 'ember-truth-helpers/helpers/or';

<template>
  <div class='wrapper' data-test-404-page>
    <div class='content'>
      {{svgJar 'cuddlyferris' class=(scopedClass 'logo')}}

      <h1 class='title' data-test-title>{{or @model.title 'Page not found'}}</h1>

      {{#if @model.details}}
        <p class='details' data-test-details>{{@model.details}}</p>
      {{/if}}

      {{#if @model.loginNeeded}}
        <button
          type='button'
          disabled={{@controller.session.loginTask.isRunning}}
          class='link button-reset text--link'
          data-test-login
          {{on 'click' (perform @controller.session.loginTask)}}
        >
          Log in with GitHub
        </button>
      {{else if @model.tryAgain}}
        <button
          type='button'
          class='link button-reset text--link'
          data-test-try-again
          {{on 'click' @controller.reload}}
        >Try Again</button>
      {{else}}
        <button type='button' class='link button-reset text--link' data-test-go-back {{on 'click' @controller.back}}>Go
          Back</button>
      {{/if}}
    </div>
  </div>
</template>
