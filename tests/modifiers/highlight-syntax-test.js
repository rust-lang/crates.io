import { module, test } from 'qunit';
import { setupRenderingTest } from 'ember-qunit';
import { render } from '@ember/test-helpers';
import { hbs } from 'ember-cli-htmlbars';

module('Modifier | highlight-syntax', function(hooks) {
  setupRenderingTest(hooks);

  test('uses Prism.js to apply syntax highlighting', async function(assert) {
    await render(hbs`
      <pre class="language-rust" {{highlight-syntax}}>
#[macro_use]
extern crate bitflags;
      </pre>
    `);
    assert.dom('.token').exists();
    assert.dom('.keyword').exists({ count: 2 });
  });

  test('accepts a `selector` argument', async function(assert) {
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
    assert.dom('.a .token').doesNotExist();
    assert.dom('.b .token').exists();
    assert.dom('.b .keyword').exists({ count: 2 });
  });
});
