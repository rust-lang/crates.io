import { module, test } from 'qunit';
import { setupRenderingTest } from 'ember-qunit';
import { render } from '@ember/test-helpers';
import hbs from 'htmlbars-inline-precompile';

module('Integration | Component | flesh-message', function(hooks) {
  setupRenderingTest(hooks);

  test('it renders', async function(assert) {
    assert.expect(2);

    await render(hbs`<FlashMessage @message="test text" />`);

    assert.dom('[data-test-flash-message]').hasText('test text');
    assert.dom('[data-test-flash-message]').isVisible();
  });
});
