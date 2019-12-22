import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: 'span',
  classNames: ['badge'],
  repository: alias('badge.attributes.repository'),
  imageUrl: computed('badge.attributes.{repository,workflow,branch,event}', function() {
    const query = Object.entries({
      branch: this.branch,
      event: this.event,
    })
      .filter(kv => kv[1] != '')
      .map(kv => kv.map(encodeURIComponent).join('='))
      .join('&');

    const base = `https://github.com/${this.repository}/workflows/${this.workflow}/badge.svg`;

    if (query != '') {
      return `${base}?${query}`;
    } else {
      return base;
    }
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
