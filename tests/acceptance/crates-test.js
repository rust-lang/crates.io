import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { click, currentURL, visit } from '@ember/test-helpers';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import axeConfig from '../axe-config';
import { title } from '../helpers/dom';
import setupMirage from '../helpers/setup-mirage';
import { percySnapshot } from 'ember-percy';

module('Acceptance | crates page', function(hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  // should match the default set in the crates controller
  const per_page = 50;

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
    assert.equal(title(), 'Crates - crates.io: Rust Package Registry');
  });

  test('visiting the crates page directly', async function(assert) {
    this.server.loadFixtures();

    await visit('/crates');
    await click('[data-test-all-crates-link]');

    assert.equal(currentURL(), '/crates');
    assert.equal(title(), 'Crates - crates.io: Rust Package Registry');
  });

  test('listing crates', async function(assert) {
    for (let i = 1; i <= per_page; i++) {
      let crate = this.server.create('crate');
      this.server.create('version', { crate });
    }

    await visit('/crates');

    assert.dom('[data-test-crates-nav] [data-test-current-rows]').hasText(`1-${per_page}`);
    assert.dom('[data-test-crates-nav] [data-test-total-rows]').hasText(`${per_page}`);
  });

  test('navigating to next page of crates', async function(assert) {
    for (let i = 1; i <= per_page + 2; i++) {
      let crate = this.server.create('crate');
      this.server.create('version', { crate });
    }
    const page_start = per_page + 1;
    const total = per_page + 2;

    await visit('/crates');
    await click('[data-test-pagination-next]');

    assert.equal(currentURL(), '/crates?page=2');
    assert.dom('[data-test-crates-nav] [data-test-current-rows]').hasText(`${page_start}-${total}`);
    assert.dom('[data-test-crates-nav] [data-test-total-rows]').hasText(`${total}`);
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
