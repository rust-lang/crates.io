import { visit } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { setupApplicationTest } from 'cargo/tests/helpers';

module('Route | crate.version | crate links', function (hooks) {
  setupApplicationTest(hooks);

  test('shows all external crate links', async function (assert) {
    let crate = this.server.create('crate', {
      name: 'foo',
      homepage: 'https://crates.io/',
      documentation: 'https://doc.rust-lang.org/cargo/getting-started/',
      repository: 'https://github.com/rust-lang/crates.io.git',
    });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/crates/foo');

    assert.dom('[data-test-homepage-link] a').hasText('crates.io').hasAttribute('href', 'https://crates.io/');

    assert
      .dom('[data-test-docs-link] a')
      .hasText('doc.rust-lang.org/cargo/getting-started')
      .hasAttribute('href', 'https://doc.rust-lang.org/cargo/getting-started/');

    assert
      .dom('[data-test-repository-link] a')
      .hasText('github.com/rust-lang/crates.io')
      .hasAttribute('href', 'https://github.com/rust-lang/crates.io.git');
  });

  test('shows no external crate links if none are set', async function (assert) {
    let crate = this.server.create('crate', { name: 'foo' });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/crates/foo');

    assert.dom('[data-test-homepage-link]').doesNotExist();
    assert.dom('[data-test-docs-link]').doesNotExist();
    assert.dom('[data-test-repository-link]').doesNotExist();
  });

  test('hide the homepage link if it is the same as the repository', async function (assert) {
    let crate = this.server.create('crate', {
      name: 'foo',
      homepage: 'https://github.com/rust-lang/crates.io',
      repository: 'https://github.com/rust-lang/crates.io',
    });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/crates/foo');

    assert.dom('[data-test-homepage-link]').doesNotExist();
    assert.dom('[data-test-docs-link]').doesNotExist();

    assert
      .dom('[data-test-repository-link] a')
      .hasText('github.com/rust-lang/crates.io')
      .hasAttribute('href', 'https://github.com/rust-lang/crates.io');
  });

  test('hide the homepage link if it is the same as the repository plus `.git`', async function (assert) {
    let crate = this.server.create('crate', {
      name: 'foo',
      homepage: 'https://github.com/rust-lang/crates.io/',
      repository: 'https://github.com/rust-lang/crates.io.git',
    });
    this.server.create('version', { crate, num: '1.0.0' });

    await visit('/crates/foo');

    assert.dom('[data-test-homepage-link]').doesNotExist();
    assert.dom('[data-test-docs-link]').doesNotExist();

    assert
      .dom('[data-test-repository-link] a')
      .hasText('github.com/rust-lang/crates.io')
      .hasAttribute('href', 'https://github.com/rust-lang/crates.io.git');
  });
});
