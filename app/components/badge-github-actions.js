import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: 'span',
  repository: alias('badge.attributes.repository'),
  imageUrl: computed('badge.attributes.{repository,workflow,branch,event}', function() {
    const url = new URL(`https://github.com/${this.repository}/workflows/${this.workflow}/badge.svg`);

    if (this.branch !== '') {
      url.searchParams.set('branch', this.branch);
    }

    if (this.event !== '') {
      url.searchParams.set('event', this.event);
    }

    return url.href;
  }),
  workflow: computed('badge.attributes.workflow', function() {
    return this.get('badge.attributes.workflow')
      .split('/')
      .map(encodeURIComponent)
      .join('/');
  }),
  branch: computed('badge.attributes.branch', function() {
    return encodeURIComponent(this.get('badge.attributes.branch') || '');
  }),
  event: computed('badge.attributes.event', function() {
    return encodeURIComponent(this.get('badge.attributes.event') || '');
  }),
  text: computed('badge', function() {
    return `GitHub Actions workflow status for the ${this.workflow} workflow`;
  }),
});
