import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import Link from 'crates-io/components/crate-sidebar/link';
import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Component | CrateSidebar::Link', function (hooks) {
  setupRenderingTest(hooks);

  test('renders title and link', async function (assert) {
    await render(<template><Link @title='Homepage' @url='https://www.rust-lang.org' /></template>);
    assert.dom('[data-test-title]').hasText('Homepage');
    assert.dom('[data-test-icon]').exists({ count: 1 }).hasAttribute('data-test-icon', 'link');
    assert.dom('[data-test-link]').hasAttribute('href', 'https://www.rust-lang.org').hasText('rust-lang.org');
  });

  test('renders GitHub icon for GitHub links', async function (assert) {
    await render(<template><Link @url='https://github.com/rust-lang/crates.io' /></template>);
    assert.dom('[data-test-icon]').exists({ count: 1 }).hasAttribute('data-test-icon', 'github');
    assert
      .dom('[data-test-link]')
      .hasAttribute('href', 'https://github.com/rust-lang/crates.io')
      .hasText('github.com/rust-lang/crates.io');
  });

  test('renders docs.rs icon for docs.rs links', async function (assert) {
    await render(<template><Link @url='https://docs.rs/tracing' /></template>);
    assert.dom('[data-test-icon]').exists({ count: 1 }).hasAttribute('data-test-icon', 'docs-rs');
    assert.dom('[data-test-link]').hasAttribute('href', 'https://docs.rs/tracing').hasText('docs.rs/tracing');
  });

  test('does not shorten HTTP links', async function (assert) {
    await render(<template><Link @url='http://www.rust-lang.org' /></template>);
    assert.dom('[data-test-link]').hasAttribute('href', 'http://www.rust-lang.org').hasText('http://www.rust-lang.org');
  });

  test('strips trailing slashes', async function (assert) {
    await render(<template><Link @url='https://www.rust-lang.org/' /></template>);
    assert.dom('[data-test-link]').hasAttribute('href', 'https://www.rust-lang.org/').hasText('rust-lang.org');
  });

  test('strips the trailing `.git` from GitHub project URLs', async function (assert) {
    await render(<template><Link @url='https://github.com/rust-lang/crates.io.git' /></template>);
    assert
      .dom('[data-test-link]')
      .hasAttribute('href', 'https://github.com/rust-lang/crates.io.git')
      .hasText('github.com/rust-lang/crates.io');
  });

  test('does not strip the trailing `.git` from other URLs', async function (assert) {
    await render(<template><Link @url='https://foo.git/' /></template>);
    assert.dom('[data-test-link]').hasAttribute('href', 'https://foo.git/').hasText('foo.git');
  });
});
