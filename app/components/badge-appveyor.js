import Component from '@ember/component';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

export default Component.extend({
    tagName: 'span',
    classNames: ['badge'],

    id: alias('badge.attributes.id'),
    repository: alias('badge.attributes.repository'),

    imageUrl: computed('badge.attributes.id', function() {
        let id = this.get('badge.attributes.id');
        let branch = this.branch;
        if (id !== undefined && id !== null) {
            return `https://ci.appveyor.com/api/projects/status/${id}/branch/${branch}?svg=true`;
        } else {
            let service = this.service;
            let repository = this.repository;

            return `https://ci.appveyor.com/api/projects/status/${service}/${repository}?svg=true&branch=${branch}`;
        }
    }),

    branch: computed('badge.attributes.branch', function() {
        return this.get('badge.attributes.branch') || 'master';
    }),

    projectName: computed('badge.attributes.project_name', function() {
        return (
            this.get('badge.attributes.project_name') || this.get('badge.attributes.repository').replace(/[_.]/g, '-')
        );
    }),

    service: computed('badge.attributes.service', function() {
        return this.get('badge.attributes.service') || 'github';
    }),

    text: computed('badge', function() {
        return `Appveyor build status for the ${this.branch} branch`;
    }),
});
