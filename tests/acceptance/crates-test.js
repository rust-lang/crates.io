import { click, currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { loadFixtures } from '@crates-io/msw/fixtures.js';
import percySnapshot from '@percy/ember';
import a11yAudit from 'ember-a11y-testing/test-support/audit';
import { getPageTitle } from 'ember-page-title/test-support';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import axeConfig from '../axe-config';

module('Acceptance | crates page', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  // should match the default set in the crates controller
  const per_page = 50;

  test('visiting the crates page from the front page', async function (assert) {
    loadFixtures(this.db);

    await visit('/');
    await click('[data-test-all-crates-link]');

    assert.strictEqual(currentURL(), '/crates');
    assert.strictEqual(getPageTitle(), 'Crates - crates.io: Rust Package Registry');

    await percySnapshot(assert);
    await a11yAudit(axeConfig);
  });

  test('visiting the crates page directly', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates');
    await click('[data-test-all-crates-link]');

    assert.strictEqual(currentURL(), '/crates');
    assert.strictEqual(getPageTitle(), 'Crates - crates.io: Rust Package Registry');
  });

  test('listing crates', async function (assert) {
    for (let i = 1; i <= per_page; i++) {
      let crate = this.db.crate.create();
      this.db.version.create({ crate });
    }

    await visit('/crates');

    assert.dom('[data-test-crates-nav] [data-test-current-rows]').hasText(`1-${per_page}`);
    assert.dom('[data-test-crates-nav] [data-test-total-rows]').hasText(`${per_page}`);
  });

  test('navigating to next page of crates', async function (assert) {
    for (let i = 1; i <= per_page + 2; i++) {
      let crate = this.db.crate.create();
      this.db.version.create({ crate });
    }
    const page_start = per_page + 1;
    const total = per_page + 2;

    await visit('/crates');
    await click('[data-test-pagination-next]');

    assert.strictEqual(currentURL(), '/crates?page=2');
    assert.dom('[data-test-crates-nav] [data-test-current-rows]').hasText(`${page_start}-${total}`);
    assert.dom('[data-test-crates-nav] [data-test-total-rows]').hasText(`${total}`);
  });

  test('crates default sort is by recent downloads', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates');

    assert.dom('[data-test-crates-sort] [data-test-current-order]').hasText('Recent Downloads');
  });

  test('downloads appears for each crate on crate list', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates');

    let formatted = Number(21_573).toLocaleString();
    assert.dom('[data-test-crate-row="0"] [data-test-downloads]').hasText(`All-Time: ${formatted}`);
  });

  test('recent downloads appears for each crate on crate list', async function (assert) {
    loadFixtures(this.db);

    await visit('/crates');

    let formatted = Number(2000).toLocaleString();
    assert.dom('[data-test-crate-row="0"] [data-test-recent-downloads]').hasText(`Recent: ${formatted}`);
  });
});
