import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
  tagName: '',
  project: alias('badge.attributes.project'),
  pipeline: alias('badge.attributes.pipeline'),

  build: computed('badge.attributes.build', function() {
    return this.get('badge.attributes.build') || '1';
  }),

  text: computed('pipeline', function() {
    return `Azure Devops build status for the ${this.pipeline} pipeline`;
  }),
});
