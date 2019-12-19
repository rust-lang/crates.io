import { module, test } from 'qunit';
import { setupRenderingTest } from 'ember-qunit';
import { render } from '@ember/test-helpers';
import { hbs } from 'ember-cli-htmlbars';

module('Integration | Component | api-token-row', function(hooks) {
  setupRenderingTest(hooks);

  test('input is focused if token is new', async function(assert) {
    // Set any properties with this.set('myProperty', 'value');
    // Handle any actions with this.set('myAction', function(val) { ... });
    this.set('api_token', {
      isNew: true,
    });

    await render(hbs`{{api-token-row api_token=api_token}}`);
    assert.dom('[data-test-focused-input]').isFocused();
  });
});
