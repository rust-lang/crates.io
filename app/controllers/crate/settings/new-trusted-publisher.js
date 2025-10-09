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
  @tracked repositoryOwner = '';
  @tracked repositoryName = '';
  @tracked workflowFilename = '';
  @tracked environment = '';
  @tracked repositoryOwnerInvalid = false;
  @tracked repositoryNameInvalid = false;
  @tracked workflowFilenameInvalid = false;

  get crate() {
    return this.model.crate;
  }

  get publishers() {
    return ['GitHub'];
  }

  get repository() {
    if (this.repositoryOwner && this.repositoryName) {
      return `${this.repositoryOwner}/${this.repositoryName}`;
    }
  }

  get verificationUrl() {
    if (this.repositoryOwner && this.repositoryName && this.workflowFilename) {
      return `https://raw.githubusercontent.com/${this.repositoryOwner}/${this.repositoryName}/HEAD/.github/workflows/${this.workflowFilename}`;
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
      repository_owner: this.repositoryOwner,
      repository_name: this.repositoryName,
      workflow_filename: this.workflowFilename,
      environment: this.environment || null,
    });

    try {
      // Save the new config on the backend
      await config.save();

      this.repositoryOwner = '';
      this.repositoryName = '';
      this.workflowFilename = '';
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
    this.repositoryOwnerInvalid = !this.repositoryOwner;
    this.repositoryNameInvalid = !this.repositoryName;
    this.workflowFilenameInvalid = !this.workflowFilename;

    return !this.repositoryOwnerInvalid && !this.repositoryNameInvalid && !this.workflowFilenameInvalid;
  }

  @action resetRepositoryOwnerValidation() {
    this.repositoryOwnerInvalid = false;
  }

  @action resetRepositoryNameValidation() {
    this.repositoryNameInvalid = false;
  }

  @action resetWorkflowFilenameValidation() {
    this.workflowFilenameInvalid = false;
  }
}
