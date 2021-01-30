import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

module('Component | CrateSidebar::Link', function (hooks) {
  setupRenderingTest(hooks);

  test('renders title and link', async function (assert) {
    await render(hbs`<CrateSidebar::Link @title="Homepage" @url="https://www.rust-lang.org" />`);
    assert.dom('[data-test-title]').hasText('Homepage');
    assert.dom('[data-test-link]').hasAttribute('href', 'https://www.rust-lang.org').hasText('rust-lang.org');
  });

  test('does not shorten HTTP links', async function (assert) {
    await render(hbs`<CrateSidebar::Link @url="http://www.rust-lang.org" />`);
    assert.dom('[data-test-link]').hasAttribute('href', 'http://www.rust-lang.org').hasText('http://www.rust-lang.org');
  });
});
