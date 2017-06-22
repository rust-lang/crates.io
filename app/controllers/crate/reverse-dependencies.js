import Ember from 'ember';
import PaginationMixin from '../../mixins/pagination';

const { computed } = Ember;

export default Ember.Controller.extend(PaginationMixin, {
    queryParams: ['page', 'per_page'],
    page: '1',
    per_page: 10,

    crateController: Ember.inject.controller('crate'),
    category: computed.alias('crateController.model'),

    totalItems: computed.readOnly('model.meta.total'),
});
