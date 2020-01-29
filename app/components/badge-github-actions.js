import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: 'span',
  repository: alias('badge.attributes.repository'),
  imageUrl: computed('badge.attributes.{repository,workflow_enc,branch,event}', function() {
    const url = new URL(`https://github.com/${this.repository}/workflows/${this.workflow_enc}/badge.svg`);

    if (this.branch !== '') {
      url.searchParams.set('branch', this.branch);
    }

    if (this.event !== '') {
      url.searchParams.set('event', this.event);
    }

    return url.href;
  }),
  url: computed('badge.attributes.{repository,workflow,branch,event}', function() {
    const url = new URL(`https://github.com/${this.repository}/actions`);

    let query = '';
    if (this.workflow !== '') {
      query += `workflow:"${this.workflow}"`;
    }

    if (this.branch !== '') {
      query += `branch:"${this.branch}"`;
    }

    if (this.event !== '') {
      query += `event:"${this.event}"`;
    }

    if (query !== '') {
      url.searchParams.set('query', query);
    }

    return url.href;
  }),
  workflow: computed('badge.attributes.workflow', function() {
    return this.get('badge.attributes.workflow');
  }),
  workflow_enc: computed('badge.attributes.workflow', function() {
    return this.get('badge.attributes.workflow')
      .split('/')
      .map(encodeURIComponent)
      .join('/');
  }),
  branch: computed('badge.attributes.branch', function() {
    return encodeURIComponent(this.get('badge.attributes.branch') || 'master');
  }),
  event: computed('badge.attributes.event', function() {
    return encodeURIComponent(this.get('badge.attributes.event') || '');
  }),
  text: computed('badge', function() {
    return `GitHub Actions workflow status for the ${this.workflow} workflow on the ${this.branch} branch`;
  }),
});
