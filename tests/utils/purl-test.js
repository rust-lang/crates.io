import { module, test } from 'qunit';

import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { addRegistryUrl } from 'crates-io/utils/purl';

module('Utils | purl', function (hooks) {
  setupWindowMock(hooks);

  module('addRegistryUrl()', function () {
    test('returns PURL unchanged for crates.io host', function (assert) {
      window.location = 'https://crates.io';

      let purl = 'pkg:cargo/serde@1.0.0';
      let result = addRegistryUrl(purl);

      assert.strictEqual(result, purl);
    });

    test('adds repository_url parameter for non-crates.io hosts', function (assert) {
      window.location = 'https://staging.crates.io';

      let purl = 'pkg:cargo/serde@1.0.0';
      let result = addRegistryUrl(purl);

      assert.strictEqual(result, 'pkg:cargo/serde@1.0.0?repository_url=https%3A%2F%2Fstaging.crates.io%2F');
    });

    test('adds repository_url parameter for custom registry hosts', function (assert) {
      window.location = 'https://my-registry.example.com';

      let purl = 'pkg:cargo/my-crate@2.5.0';
      let result = addRegistryUrl(purl);

      assert.strictEqual(result, 'pkg:cargo/my-crate@2.5.0?repository_url=https%3A%2F%2Fmy-registry.example.com%2F');
    });

    test('appends repository_url parameter when PURL already has query parameters', function (assert) {
      window.location = 'https://staging.crates.io';

      let purl = 'pkg:cargo/serde@1.0.0?arch=x86_64';
      let result = addRegistryUrl(purl);

      assert.strictEqual(result, 'pkg:cargo/serde@1.0.0?arch=x86_64&repository_url=https%3A%2F%2Fstaging.crates.io%2F');
    });

    test('properly URL encodes the repository URL', function (assert) {
      window.location = 'https://registry.example.com:8080';

      let purl = 'pkg:cargo/test@1.0.0';
      let result = addRegistryUrl(purl);

      assert.strictEqual(result, 'pkg:cargo/test@1.0.0?repository_url=https%3A%2F%2Fregistry.example.com%3A8080%2F');
    });

    test('handles PURL with complex qualifiers', function (assert) {
      window.location = 'https://private.registry.co';

      let purl = 'pkg:cargo/complex@1.0.0?os=linux&arch=amd64';
      let result = addRegistryUrl(purl);

      assert.strictEqual(
        result,
        'pkg:cargo/complex@1.0.0?os=linux&arch=amd64&repository_url=https%3A%2F%2Fprivate.registry.co%2F',
      );
    });
  });
});
