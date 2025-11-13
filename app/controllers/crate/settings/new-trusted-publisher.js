import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';
import Ember from 'ember';

import { rawTimeout, restartableTask, task } from 'ember-concurrency';

export default class NewTrustedPublisherController extends Controller {
  @service notifications;
  @service store;
  @service router;

  @tracked publisher = 'GitHub';
  @tracked namespace = '';
  @tracked project = '';
  @tracked workflow = '';
  @tracked environment = '';
  @tracked namespaceInvalid = false;
  @tracked projectInvalid = false;
  @tracked workflowInvalid = false;

  get crate() {
    return this.model.crate;
  }

  get publishers() {
    return ['GitHub'];
  }

  get repository() {
    if (this.namespace && this.project) {
      return `${this.namespace}/${this.project}`;
    }
  }

  get verificationUrl() {
    if (this.namespace && this.project && this.workflow) {
      return `https://raw.githubusercontent.com/${this.namespace}/${this.project}/HEAD/.github/workflows/${this.workflow}`;
    }
  }

  verifyWorkflowTask = restartableTask(async () => {
    let timeout = Ember.testing ? 0 : 500;
    await rawTimeout(timeout);

    let { verificationUrl } = this;
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

  saveConfigTask = task(async () => {
    if (!this.validate()) return;

    let config = this.store.createRecord('trustpub-github-config', {
      crate: this.crate,
      repository_owner: this.namespace,
      repository_name: this.project,
      workflow_filename: this.workflow,
      environment: this.environment || null,
    });

    try {
      // Save the new config on the backend
      await config.save();

      this.namespace = '';
      this.project = '';
      this.workflow = '';
      this.environment = '';

      // Navigate back to the crate settings page
      this.notifications.success('Trusted Publishing configuration added successfully');
      this.router.transitionTo('crate.settings', this.crate.id);
    } catch (error) {
      // Notify the user
      let message = 'An error has occurred while adding the Trusted Publishing configuration';

      let detail = error.errors?.[0]?.detail;
      if (detail && !detail.startsWith('{')) {
        message += `: ${detail}`;
      }

      this.notifications.error(message);
    }
  });

  validate() {
    this.namespaceInvalid = !this.namespace;
    this.projectInvalid = !this.project;
    this.workflowInvalid = !this.workflow;

    return !this.namespaceInvalid && !this.projectInvalid && !this.workflowInvalid;
  }

  @action resetNamespaceValidation() {
    this.namespaceInvalid = false;
  }

  @action resetProjectValidation() {
    this.projectInvalid = false;
  }

  @action resetWorkflowValidation() {
    this.workflowInvalid = false;
  }
}
