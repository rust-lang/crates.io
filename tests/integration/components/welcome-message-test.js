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
    const user = this.server.create('user');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert
      .dom('[data-test-welcome-message]')
      .hasText('Welcome to crates.io! Visit account settings to verify your email address and create an API token!');
    assert.dom('[data-test-welcome-message]').isVisible();
  });

  test('it show reminder about email only if user has tokens', async function(assert) {
    assert.expect(2);
    const user = this.server.create('user', 'withTokens');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert
      .dom('[data-test-welcome-message]')
      .hasText('Welcome to crates.io! Visit account settings to verify your email address!');
    assert.dom('[data-test-welcome-message]').isVisible();
  });

  test('it show reminder about tokens only if user has verified email', async function(assert) {
    assert.expect(2);
    const user = this.server.create('user', 'withVerifiedEmail');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert
      .dom('[data-test-welcome-message]')
      .hasText('Welcome to crates.io! Visit account settings to create an API token!');
    assert.dom('[data-test-welcome-message]').isVisible();
  });

  test('it not shows if user has tokens and verified email', async function(assert) {
    assert.expect(1);
    const user = this.server.create('user', 'withTokens', 'withVerifiedEmail');

    this.session = this.owner.lookup('service:session');
    this.session.loginUser(user);

    await render(hbs`{{welcome-message}}`);

    assert.dom('[data-test-welcome-message]').isNotVisible();
  });
});
