import { currentURL } from '@ember/test-helpers';
import { module, test } from 'qunit';

import percySnapshot from '@percy/ember';
import { http, HttpResponse } from 'msw';

import { setupApplicationTest } from 'crates-io/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Dashboard', function (hooks) {
  setupApplicationTest(hooks, { msw: true });

  test('shows "page requires authentication" error when not logged in', async function (assert) {
    await visit('/dashboard');
    assert.strictEqual(currentURL(), '/dashboard');
    assert.dom('[data-test-title]').hasText('This page requires authentication');
    assert.dom('[data-test-login]').exists();
  });

  test('shows the dashboard when logged in', async function (assert) {
    let user = this.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    this.authenticateAs(user);

    {
      let crate = this.db.crate.create({ name: 'rand' });
      this.db.version.create({ crate, num: '0.5.0' });
      this.db.version.create({ crate, num: '0.6.0' });
      this.db.version.create({ crate, num: '0.7.0' });
      this.db.version.create({ crate, num: '0.7.1' });
      this.db.version.create({ crate, num: '0.7.2' });
      this.db.version.create({ crate, num: '0.7.3' });
      this.db.version.create({ crate, num: '0.8.0' });
      this.db.version.create({ crate, num: '0.8.1' });
      this.db.version.create({ crate, num: '0.9.0' });
      this.db.version.create({ crate, num: '1.0.0' });
      this.db.version.create({ crate, num: '1.1.0' });
      user = this.db.user.update({
        where: { id: { equals: user.id } },
        data: { followedCrates: [...user.followedCrates, crate] },
      });
    }

    {
      let crate = this.db.crate.create({ name: 'nanomsg' });
      this.db.crateOwnership.create({ crate, user });
      this.db.version.create({ crate, num: '0.1.0' });
      user = this.db.user.update({
        where: { id: { equals: user.id } },
        data: { followedCrates: [...user.followedCrates, crate] },
      });
    }

    this.worker.use(http.get(`/api/v1/users/${user.id}/stats`, () => HttpResponse.json({ total_downloads: 3892 })));

    await visit('/dashboard');
    assert.strictEqual(currentURL(), '/dashboard');
    await percySnapshot(assert);
  });
});
