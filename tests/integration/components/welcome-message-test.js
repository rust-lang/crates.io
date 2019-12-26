import { module, test } from 'qunit';
import { setupRenderingTest } from 'ember-qunit';
import { render } from '@ember/test-helpers';
import hbs from 'htmlbars-inline-precompile';
import setupMirage from '../../helpers/setup-mirage';

module('Integration | Component | welcome-message', function(hooks) {
  setupRenderingTest(hooks);
  setupMirage(hooks);

  test('it renders', async function(assert) {
    assert.expect(2);
    const user = this.server.create('user', {});

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert.equal(
      this.element.textContent.trim(),
      'Welcome to crates.io! Visit account settings to verify your email address and create an API token!',
      'should show right message',
    );
    assert.equal(
      this.element.querySelector('#welcome-message').className,
      'shown info ember-view',
      'should have right class',
    );
  });

  test('it show reminder about email only if user has tokens', async function(assert) {
    assert.expect(2);
    const user = this.server.create('user', 'withTokens');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert.equal(
      this.element.textContent.trim(),
      'Welcome to crates.io! Visit account settings to verify your email address!',
      'should show right message',
    );
    assert.equal(
      this.element.querySelector('#welcome-message').className,
      'shown info ember-view',
      'should have right class',
    );
  });

  test('it show reminder about tokens only if user has verified email', async function(assert) {
    assert.expect(2);
    const user = this.server.create('user', 'withVerifiedEmail');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert.equal(
      this.element.textContent.trim(),
      'Welcome to crates.io! Visit account settings to create an API token!',
      'should show right message',
    );
    assert.equal(
      this.element.querySelector('#welcome-message').className,
      'shown info ember-view',
      'should have right class',
    );
  });

  test('it not shows if user has tokens and verified email', async function(assert) {
    assert.expect(1);
    const user = this.server.create('user', 'withTokens', 'withVerifiedEmail');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert.equal(
      this.element.querySelector('#welcome-message').className,
      'info ember-view',
      'should have right class',
    );
  });
});
