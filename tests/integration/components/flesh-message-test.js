import { module, test } from 'qunit';
import { setupRenderingTest } from 'ember-qunit';
import { render } from '@ember/test-helpers';
import hbs from 'htmlbars-inline-precompile';

module('Integration | Component | flesh-message', function(hooks) {
  setupRenderingTest(hooks);

  test('it renders', async function(assert) {
    assert.expect(2);

    this.flashMessages = this.owner.lookup('service:flashMessages');
    this.flashMessages.show('test text');

    await render(hbs`{{flash-message}}`);

    assert.equal(this.element.textContent.trim(), 'test text', 'should show right message');
    assert.equal(this.element.querySelector('#flash').className, 'shown warning ember-view', 'should have right class');
  });

  test('it renders with right passed type', async function(assert) {
    assert.expect(1);

    this.flashMessages = this.owner.lookup('service:flashMessages');
    this.flashMessages.show('test', { type: 'info' });

    await render(hbs`{{flash-message}}`);

    assert.equal(this.element.querySelector('#flash').className, 'shown info ember-view', 'should have right class');
  });
});
