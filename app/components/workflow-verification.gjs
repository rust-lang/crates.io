/* eslint-disable ember/no-at-ember-render-modifiers */
import { isTesting } from '@ember/debug';
import didInsert from '@ember/render-modifiers/modifiers/did-insert';
import didUpdate from '@ember/render-modifiers/modifiers/did-update';
import Component from '@glimmer/component';

import { rawTimeout, restartableTask } from 'ember-concurrency';
import perform from 'ember-concurrency/helpers/perform';
import or from 'ember-truth-helpers/helpers/or';

export default class WorkflowVerificationComponent extends Component {
  verifyWorkflowTask = restartableTask(async () => {
    let timeout = isTesting() ? 0 : 500;
    await rawTimeout(timeout);

    let { url } = this.args;
    if (!url) return null;

    try {
      let response = await fetch(url, { method: 'HEAD' });

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

  get isRunning() {
    return this.verifyWorkflowTask.isRunning;
  }

  get isSuccess() {
    return this.verifyWorkflowTask.last?.value === 'success';
  }

  get isNotFound() {
    return this.verifyWorkflowTask.last?.value === 'not-found';
  }

  get isError() {
    return this.verifyWorkflowTask.last?.value === 'error';
  }

  get status() {
    if (this.isRunning) {
      return 'running';
    } else if (this.isSuccess) {
      return 'success';
    } else if (this.isNotFound) {
      return 'not-found';
    } else if (this.isError) {
      return 'error';
    } else {
      return 'initial';
    }
  }

  <template>
    <div
      class='workflow-verification
        {{if this.isSuccess "workflow-verification--success"}}
        {{if (or this.isNotFound this.isError) "workflow-verification--warning"}}'
      data-test-workflow-verification={{this.status}}
      {{didInsert (perform this.verifyWorkflowTask)}}
      {{didUpdate (perform this.verifyWorkflowTask @url)}}
    >
      {{#if this.isRunning}}
        Verifying...
      {{else if this.isSuccess}}
        ✓ Workflow file found at
        <a href='{{@url}}' target='_blank' rel='noopener noreferrer'>{{@url}}</a>
      {{else if this.isNotFound}}
        ⚠ Workflow file not found at
        <a href='{{@url}}' target='_blank' rel='noopener noreferrer'>{{@url}}</a>
      {{else if this.isError}}
        ⚠ Could not verify workflow file at
        <a href='{{@url}}' target='_blank' rel='noopener noreferrer'>{{@url}}</a>
        (network error)
      {{else}}
        The workflow
        {{@fieldType}}
        will be verified once all necessary fields are filled.
      {{/if}}
    </div>
  </template>
}
