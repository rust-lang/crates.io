import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../../helpers/visit-ignoring-abort';

module('Route | crate.settings', hooks => {
  setupApplicationTest(hooks);

  function prepare(context) {
    const user = context.db.user.create();

    const crate = context.db.crate.create({ name: 'foo' });
    context.db.version.create({ crate });
    context.db.crateOwnership.create({ crate, user });

    return { crate, user };
  }

  test('unauthenticated', async function (assert) {
    const crate = this.db.crate.create({ name: 'foo' });
    this.db.version.create({ crate });

    await visit('/crates/foo/settings');
    assert.strictEqual(currentURL(), '/crates/foo/settings');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('not an owner', async function (assert) {
    const { crate } = prepare(this);

    const otherUser = this.db.user.create();
    this.authenticateAs(otherUser);

    await visit(`/crates/${crate.name}/settings`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
    assert.dom('[data-test-title]').hasText('This page is only accessible by crate owners');
    assert.dom('[data-test-go-back]').exists();
  });

  test('happy path', async function (assert) {
    const { crate, user } = prepare(this);
    this.authenticateAs(user);

    await visit(`/crates/${crate.name}/settings`);
    assert.strictEqual(currentURL(), `/crates/${crate.name}/settings`);
    // This is the Add Owner button.
    assert.dom('[data-test-save-button]').exists();
    assert.dom('[data-test-owners]').exists();
    assert.dom(`[data-test-owner-user="${user.login}"]`).exists();
    assert.dom('[data-test-remove-owner-button]').exists();
    assert.dom('[data-test-delete-button]').exists();
  });
});
