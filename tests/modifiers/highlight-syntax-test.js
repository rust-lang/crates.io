import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'cargo/tests/helpers';

module('Modifier | highlight-syntax', function (hooks) {
  setupRenderingTest(hooks);

  test('uses Prism.js to apply syntax highlighting', async function (assert) {
    await render(hbs`
      <pre class="language-rust" {{highlight-syntax}}>
#[macro_use]
extern crate bitflags;
      </pre>
    `);

    assert.dom('.hljs-meta').exists();
    assert.dom('.hljs-keyword').exists({ count: 2 });
  });

  test('accepts a `selector` argument', async function (assert) {
    await render(hbs`
      <div {{highlight-syntax selector=".b"}}>
        <pre class="language-rust a">
  #[macro_use]
  extern crate bitflags;
        </pre>
        <pre class="language-rust b">
  #[macro_use]
  extern crate bitflags;
        </pre>
      </div>
    `);
    assert.dom('.a .hljs-meta').doesNotExist();
    assert.dom('.b .hljs-meta').exists();
    assert.dom('.b .hljs-keyword').exists({ count: 2 });
  });
});
