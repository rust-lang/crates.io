import { on } from '@ember/modifier';

import perform from 'ember-concurrency/helpers/perform';

<template>
  {{#if @version.yanked}}
    <button
      type='button'
      ...attributes
      data-test-version-unyank-button={{@version.num}}
      disabled={{@version.unyankTask.isRunning}}
      {{on 'click' (perform @version.unyankTask)}}
    >
      {{#if @version.unyankTask.isRunning}}
        Unyanking...
      {{else}}
        Unyank
      {{/if}}
    </button>
  {{else}}
    <button
      type='button'
      ...attributes
      data-test-version-yank-button={{@version.num}}
      disabled={{@version.yankTask.isRunning}}
      {{on 'click' (perform @version.yankTask)}}
    >
      {{#if @version.yankTask.isRunning}}
        Yanking...
      {{else}}
        Yank
      {{/if}}
    </button>
  {{/if}}
</template>
