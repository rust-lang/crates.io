import { Factory, faker } from 'ember-cli-mirage';
import { dasherize } from '@ember/string';

export default Factory.extend({
    category(i) {
        return `Category ${i}`;
    },

    id() {
        return dasherize(this.category);
    },

    slug() {
        return dasherize(this.category);
    },

    description: () => faker.lorem.sentence(),
    created_at: () => faker.date.past(),
    crates_cnt: () => faker.random.number({ max: 5000 }),
});
