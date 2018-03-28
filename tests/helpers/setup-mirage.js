import setupMirage from 'ember-cli-mirage/test-support/setup-mirage';
import { faker } from 'ember-cli-mirage';

export default function(hooks) {
    setupMirage(hooks);

    // To have deterministic visual tests, the seed has to be constant
    hooks.beforeEach(function() {
        faker.seed(12345);
    });
}
