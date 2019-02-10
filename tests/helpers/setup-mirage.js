import setupMirage from 'ember-cli-mirage/test-support/setup-mirage';
import { faker } from 'ember-cli-mirage';
import timekeeper from 'timekeeper';

export default function(hooks) {
    setupMirage(hooks);

    // To have deterministic visual tests, the seed has to be constant
    hooks.beforeEach(function() {
        faker.seed(12345);
        timekeeper.freeze(new Date('11/20/2017 12:00'));
    });

    hooks.afterEach(function() {
        timekeeper.reset();
    });
}
