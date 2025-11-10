/* eslint-disable ember/no-at-ember-render-modifiers */
import didInsert from '@ember/render-modifiers/modifiers/did-insert';
import didUpdate from '@ember/render-modifiers/modifiers/did-update';
import Component from '@glimmer/component';
import Ember from 'ember';

import { rawTimeout, restartableTask } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';
import eq from 'ember-truth-helpers/helpers/eq';
import not from 'ember-truth-helpers/helpers/not';

export default class WorkflowVerificationComponent extends Component {
  verifyWorkflowTask = restartableTask(async () => {
    let timeout = Ember.testing ? 0 : 500;
    await rawTimeout(timeout);

    let { verificationUrl } = this.args;
    if (!verificationUrl) return null;

    try {
      let response = await fetch(verificationUrl, { method: 'HEAD' });

      if (response.ok) {
        return 'success';
      } else if (response.status === 404) {
        return 'not-found';
      } else {
        return 'error';
      }
    } catch {
      return 'error';
    }
  });

  <template>
    <div
      {{didInsert (perform this.verifyWorkflowTask)}}
      {{didUpdate (perform this.verifyWorkflowTask @verificationUrl)}}
    >
      {{#if (not @verificationUrl)}}
        <div class='workflow-verification' data-test-workflow-verification='initial'>
          The workflow
          {{@fieldType}}
          will be verified once all necessary fields are filled.
        </div>
      {{else if (eq this.verifyWorkflowTask.last.value 'success')}}
        <div class='workflow-verification workflow-verification--success' data-test-workflow-verification='success'>
          ✓ Workflow file found at
          <a href='{{@verificationUrl}}' target='_blank' rel='noopener noreferrer'>
            {{@verificationUrl}}
          </a>
        </div>
      {{else if (eq this.verifyWorkflowTask.last.value 'not-found')}}
        <div class='workflow-verification workflow-verification--warning' data-test-workflow-verification='not-found'>
          ⚠ Workflow file not found at
          <a href='{{@verificationUrl}}' target='_blank' rel='noopener noreferrer'>
            {{@verificationUrl}}
          </a>
        </div>
      {{else if (eq this.verifyWorkflowTask.last.value 'error')}}
        <div class='workflow-verification workflow-verification--warning' data-test-workflow-verification='error'>
          ⚠ Could not verify workflow file at
          <a href='{{@verificationUrl}}' target='_blank' rel='noopener noreferrer'>
            {{@verificationUrl}}
          </a>
          (network error)
        </div>
      {{else}}
        <div class='workflow-verification' data-test-workflow-verification='verifying'>
          Verifying...
        </div>
      {{/if}}
    </div>
  </template>
}
