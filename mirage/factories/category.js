import { Factory, faker } from 'ember-cli-mirage';
import Ember from 'ember';

export default Factory.extend({
    category(i) {
        return `Category ${i}`;
    },

    id() {
        return Ember.String.dasherize(this.category);
    },

    slug() {
        return Ember.String.dasherize(this.category);
    },

    description: () => faker.lorem.sentence(),
    created_at: () => faker.date.past(),
    crates_cnt: () => faker.random.number({ max: 5000 }),
});
