import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { click, currentURL, visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | crates page', function(hooks) {
    setupApplicationTest(hooks);
    setupMirage(hooks);

    test('is accessible', async function(assert) {
        assert.expect(0);

        this.server.loadFixtures();

        await visit('/');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('/crates is accessible', async function(assert) {
        assert.expect(0);

        this.server.loadFixtures();

        await visit('/crates');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('visiting the crates page from the front page', async function(assert) {
        this.server.loadFixtures();

        await visit('/');
        await click('[data-test-all-crates-link]');

        assert.equal(currentURL(), '/crates');
        assert.equal(document.title, 'Crates - Cargo: packages for Rust');
    });

    test('visiting the crates page directly', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates');
        await click('[data-test-all-crates-link]');

        assert.equal(currentURL(), '/crates');
        assert.equal(document.title, 'Crates - Cargo: packages for Rust');
    });

    test('listing crates', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates');

        assert.dom('[data-test-crates-nav] [data-test-current-rows]').hasText('1-10');
        assert.dom('[data-test-crates-nav] [data-test-total-rows]').hasText('19');
    });

    test('navigating to next page of crates', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates');
        await click('[data-test-pagination-next]');

        assert.equal(currentURL(), '/crates?page=2');
        assert.dom('[data-test-crates-nav] [data-test-current-rows]').hasText('11-19');
        assert.dom('[data-test-crates-nav] [data-test-total-rows]').hasText('19');
    });

    test('crates default sort is alphabetical', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates');

        assert.dom('[data-test-crates-sort] [data-test-current-order]').hasText('Alphabetical');
    });

    test('downloads appears for each crate on crate list', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates');
        assert.dom('[data-test-crate-row="0"] [data-test-downloads]').hasText('All-Time: 497');
    });

    test('recent downloads appears for each crate on crate list', async function(assert) {
        this.server.loadFixtures();

        await visit('/crates');
        assert.dom('[data-test-crate-row="0"] [data-test-recent-downloads]').hasText('Recent: 497');
    });
});
