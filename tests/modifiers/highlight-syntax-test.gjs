import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import highlightSyntax from 'crates-io/modifiers/highlight-syntax';
import { setupRenderingTest } from 'crates-io/tests/helpers';

module('Modifier | highlight-syntax', function (hooks) {
  setupRenderingTest(hooks);

  test('uses Prism.js to apply syntax highlighting', async function (assert) {
    await render(
      <template>
        <pre class='language-rust' {{highlightSyntax}}>
#[macro_use]
extern crate bitflags;
      </pre>
      </template>,
    );

    assert.dom('.hljs-meta').exists();
    assert.dom('.hljs-keyword').exists({ count: 2 });
  });

  test('accepts a `selector` argument', async function (assert) {
    await render(
      <template>
        <div {{highlightSyntax selector='.b'}}>
          <pre class='language-rust a'>
  #[macro_use]
  extern crate bitflags;
        </pre>
          <pre class='language-rust b'>
  #[macro_use]
  extern crate bitflags;
        </pre>
        </div>
      </template>,
    );
    assert.dom('.a .hljs-meta').doesNotExist();
    assert.dom('.b .hljs-meta').exists();
    assert.dom('.b .hljs-keyword').exists({ count: 2 });
  });
});
