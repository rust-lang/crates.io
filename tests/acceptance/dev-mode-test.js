import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

import { visit } from '../helpers/visit-ignoring-abort';

let s = new URLSearchParams(window.location.search);
if (s.has('devmode')) {
  /**
   * This module will only be available if the "Development Mode" option of QUnit is chosen
   * @link http://localhost:4200/tests/?notrycatch&devmode&filter=Development%20Mode
   */
  module('Development Mode', function (hooks) {
    setupApplicationTest(hooks);

    test('authenticated', async function () {
      let user = this.server.create('user');
      this.authenticateAs(user);

      let crate = this.server.create('crate', { name: 'foo' });
      this.server.create('version', { crate, num: '0.1.0' });
      this.server.create('crate-ownership', { crate, user });

      crate = this.server.create('crate', { name: 'bar' });
      this.server.create('version', { crate, num: '1.0.0' });

      let router = this.owner.lookup('service:router');
      router.on('routeDidChange', () => {
        console.log(`URL: ${router.currentURL}`);
      });

      await visit('/');
      // eslint-disable-next-line ember/no-pause-test
      await this.pauseTest();
    });
  });
}
