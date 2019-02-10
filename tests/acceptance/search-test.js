import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { fillIn, currentURL, triggerEvent, visit, blur } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { triggerKeyDown, triggerKeyPress } from 'ember-keyboard';
import axeConfig from '../axe-config';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | search', function(hooks) {
    setupApplicationTest(hooks);
    setupMirage(hooks);

    test('/search?q=rust is accessible', async function(assert) {
        assert.expect(0);

        this.server.loadFixtures();

        await visit('/');
        percySnapshot(assert);

        await a11yAudit(axeConfig);
    });

    test('searching for "rust"', async function(assert) {
        this.server.loadFixtures();

        await visit('/');
        await fillIn('[data-test-search-input]', 'rust');
        await triggerEvent('[data-test-search-form]', 'submit');

        assert.equal(currentURL(), '/search?q=rust');
        assert.equal(document.title, "Search Results for 'rust' - Cargo: packages for Rust");

        assert.dom('[data-test-heading]').hasText("Search Results for 'rust'");
        assert.dom('[data-test-search-nav]').hasText('Displaying 1-8 of 8 total results');
        assert
            .dom('[data-test-search-sort]')
            .hasText('Sort by Relevance Relevance All-Time Downloads Recent Downloads Recent Updates');
        assert.dom('[data-test-crate-row="0"] [data-test-crate-link]').hasText('kinetic-rust');
        assert.dom('[data-test-crate-row="0"] [data-test-version-badge]').hasAttribute('alt', '0.0.16');

        assert
            .dom('[data-test-crate-row="0"] [data-test-description]')
            .hasText('A Kinetic protocol library written in Rust');
        assert.dom('[data-test-crate-row="0"] [data-test-downloads]').hasText('All-Time: 225');
        assert.dom('[data-test-crate-row="0"] [data-test-badge="maintenance"]').exists();
    });

    test('searching for "rust" from query', async function(assert) {
        this.server.loadFixtures();

        await visit('/search?q=rust');

        assert.equal(currentURL(), '/search?q=rust');
        assert.equal(document.title, "Search Results for 'rust' - Cargo: packages for Rust");

        assert.dom('[data-test-search-input]').hasValue('rust');
        assert.dom('[data-test-heading]').hasText("Search Results for 'rust'");
        assert.dom('[data-test-search-nav]').hasText('Displaying 1-8 of 8 total results');
    });

    test('pressing S key to focus the search bar', async function(assert) {
        this.server.loadFixtures();

        await visit('/');

        await blur('[data-test-search-input]');
        await triggerKeyPress('KeyA');
        assert.dom('[data-test-search-input]').isNotFocused();

        await blur('[data-test-search-input]');
        await triggerKeyPress('KeyS');
        assert.dom('[data-test-search-input]').isFocused();

        await blur('[data-test-search-input]');
        await triggerKeyDown('KeyS');
        assert.dom('[data-test-search-input]').isFocused();

        await blur('[data-test-search-input]');
        await triggerKeyDown('shift+KeyS');
        assert.dom('[data-test-search-input]').isFocused();
    });

    test('check search results are by default displayed by relevance', async function(assert) {
        this.server.loadFixtures();

        await visit('/');
        await fillIn('[data-test-search-input]', 'rust');
        await triggerEvent('[data-test-search-form]', 'submit');

        assert.dom('[data-test-search-sort] [data-test-current-order]').hasText('Relevance');
    });
});
