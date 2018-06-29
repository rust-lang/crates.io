import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL, visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | front page', function(hooks) {
    setupApplicationTest(hooks);
    setupMirage(hooks);

    test('is accessible', async function(assert) {
        assert.expect(0);

        this.server.loadFixtures();

        await visit('/');
        await a11yAudit(axeConfig);
    });

    test('visiting /', async function(assert) {
        this.server.loadFixtures();

        await visit('/');

        assert.equal(currentURL(), '/');
        assert.equal(document.title, 'Cargo: packages for Rust');

        assert.dom('[data-test-install-cargo-link]').exists();
        assert.dom('[data-test-all-crates-link]').exists();
        assert.dom('[data-test-login-link]').exists();

        assert.dom('[data-test-total-downloads]').hasText('122,669');
        assert.dom('[data-test-total-crates]').hasText('19');

        assert.dom('[data-test-new-crates] [data-test-crate-link="0"]').hasText('Inflector (0.1.6)');
        assert.dom('[data-test-new-crates] [data-test-crate-link="0"]').hasAttribute('href', '/crates/Inflector');

        assert.dom('[data-test-most-downloaded] [data-test-crate-link="0"]').hasText('serde (0.6.1)');
        assert.dom('[data-test-most-downloaded] [data-test-crate-link="0"]').hasAttribute('href', '/crates/serde');

        assert.dom('[data-test-just-updated] [data-test-crate-link="0"]').hasText('nanomsg (0.7.0-alpha)');
        assert.dom('[data-test-just-updated] [data-test-crate-link="0"]').hasAttribute('href', '/crates/nanomsg');

        percySnapshot(assert);
    });
});
