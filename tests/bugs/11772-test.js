import { click, currentURL, visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

module('Bug #11772', function (hooks) {
  setupApplicationTest(hooks);

  async function prepare(context) {
    let { db } = context;

    // Create a crate that will appear in "New Crates" section
    let newCrate = await db.crate.create({ name: 'test-crate' });
    await db.version.create({ crate: newCrate, num: '1.2.3' });
  }

  test('crate versions should remain correct after navigating back from crate details', async function (assert) {
    await prepare(this);

    // Visit homepage
    await visit('/');
    assert.strictEqual(currentURL(), '/');

    // Verify initial correct version displays
    assert.dom('[data-test-new-crates] [data-test-crate-link]').containsText('test-crate v1.2.3');
    assert.dom('[data-test-just-updated] [data-test-crate-link]').containsText('test-crate v1.2.3');

    // Click on a crate to navigate to its details page
    await click('[data-test-new-crates] [data-test-crate-link]');

    // Verify we're on the crate details page
    assert.strictEqual(currentURL(), '/crates/test-crate');

    await visit('/'); // Re-visit to simulate the back navigation

    // Versions should still be displayed correctly, not v0.0.0
    assert.dom('[data-test-new-crates] [data-test-crate-link]').containsText('test-crate v1.2.3');
    assert.dom('[data-test-just-updated] [data-test-crate-link]').containsText('test-crate v1.2.3');
  });

  test('crates with actual v0.0.0 versions should display correctly', async function (assert) {
    let { db } = this;

    // Create a crate with an actual v0.0.0 version
    let zeroCrate = await db.crate.create({ name: 'test-zero-crate' });
    await db.version.create({ crate: zeroCrate, num: '0.0.0' });

    // Visit homepage
    await visit('/');
    assert.strictEqual(currentURL(), '/');

    // Should correctly display v0.0.0 for crates that actually have that version
    assert.dom('[data-test-new-crates] [data-test-crate-link]').containsText('test-zero-crate v0.0.0');

    // Click on the crate to navigate to its details page
    await click('[data-test-new-crates] [data-test-crate-link]');

    // Verify we're on the crate details page
    assert.strictEqual(currentURL(), '/crates/test-zero-crate');

    await visit('/'); // Re-visit to simulate the back navigation

    // Should still display v0.0.0 correctly (this is the intended behavior)
    assert.dom('[data-test-new-crates] [data-test-crate-link]').containsText('test-zero-crate v0.0.0');
  });
});
