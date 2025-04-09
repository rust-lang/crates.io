import { click, find, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import format from 'date-fns/format';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | sudo', function (hooks) {
  setupApplicationTest(hooks);

  function prepare(context, isAdmin) {
    const user = context.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      isAdmin,
    });

    const crate = context.db.crate.create({
      name: 'foo',
      newest_version: '0.1.0',
    });

    const version = context.db.version.create({
      crate,
      num: '0.1.0',
    });

    context.authenticateAs(user);
    return { user, crate, version };
  }

  test('non-admin users do not see any controls', async function (assert) {
    prepare(this, false);

    await visit('/crates/foo/versions');

    // Test the various header elements.
    assert.dom('[data-test-wizard-hat]').doesNotExist();
    assert.dom('[data-test-disable-admin-actions]').doesNotExist();
    assert.dom('[data-test-enable-admin-actions]').doesNotExist();

    // Assert that there's no yank button, disabled, enabled, or in any state.
    assert.dom('[data-test-version-yank-button="0.1.0"]').doesNotExist();
  });

  test('admin user is not initially in sudo mode', async function (assert) {
    prepare(this, true);

    await visit('/crates/foo/versions');

    // Test the various header elements.
    assert.dom('[data-test-wizard-hat]').doesNotExist();
    assert.dom('[data-test-disable-admin-actions]').doesNotExist();
    assert.dom('[data-test-enable-admin-actions]').exists();

    // Test that the fieldset is present and disabled.
    assert.dom('[data-test-placeholder-fieldset]').exists().isDisabled();

    // From the perspective of the actual button, it isn't disabled, even though
    // the fieldset effectively makes it unclickable.
    assert.dom('[data-test-version-yank-button="0.1.0"]').exists();
  });

  test('admin user can enter sudo mode', async function (assert) {
    prepare(this, true);

    await visit('/crates/foo/versions');

    const untilAbout = Date.now() + 6 * 60 * 60 * 1000;
    await click('[data-test-enable-admin-actions]');

    // Test the various header elements.
    assert.dom('[data-test-wizard-hat]').exists();
    assert.dom('[data-test-disable-admin-actions]').exists();
    assert.dom('[data-test-enable-admin-actions]').doesNotExist();

    // Test that the expiry time is sensible. We'll allow a minute either way in
    // case of slow tests or slightly wonky clocks.
    const disable = find('[data-test-disable-admin-actions] > div');
    let seen = 0;
    for (const ts of [untilAbout - 60 * 1000, untilAbout, untilAbout + 60 * 1000]) {
      const time = format(new Date(ts), 'HH:mm');
      if (disable.textContent.includes(time)) {
        seen += 1;
      }
    }
    assert.strictEqual(seen, 1);

    // Test that the fieldset is not present.
    assert.dom('[data-test-placeholder-fieldset]').doesNotExist();
    assert.dom('[data-test-version-yank-button="0.1.0"]').exists();
  });

  test('admin can yank a crate in sudo mode', async function (assert) {
    prepare(this, true);

    await visit('/crates/foo/versions');
    await click('[data-test-enable-admin-actions]');

    await click('[data-test-version-yank-button="0.1.0"]');

    await waitFor('[data-test-version-unyank-button="0.1.0"]');
    const crate = this.db.crate.findFirst({ where: { name: { equals: 'foo' } } });
    const version = this.db.version.findFirst({ crate: { id: { equals: crate.id } }, num: { equals: '0.1.0' } });
    assert.true(version.yanked, 'The version should be yanked');
    assert.dom('[data-test-version-unyank-button="0.1.0"]').exists();
    await click('[data-test-version-unyank-button="0.1.0"]');
    const updatedVersion = this.db.version.findFirst({ crate: { id: { equals: crate.id } }, num: { equals: '0.1.0' } });
    assert.false(updatedVersion.yanked, 'The version should be unyanked');

    await waitFor('[data-test-version-yank-button="0.1.0"]');
    assert.dom('[data-test-version-yank-button="0.1.0"]').exists();
  });
});
