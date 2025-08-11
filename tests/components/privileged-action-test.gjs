import { render } from '@ember/test-helpers';
import { module, test } from 'qunit';

import { hbs } from 'ember-cli-htmlbars';

import { setupRenderingTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';

module('Component | PrivilegedAction', hooks => {
  setupRenderingTest(hooks);
  setupMsw(hooks);

  hooks.beforeEach(function () {
    // Adds a utility function that renders a PrivilegedAction with all the
    // possible content blocks.
    this.renderComponent = async function (userAuthorised) {
      this.userAuthorised = userAuthorised;
      await render(hbs`
      <PrivilegedAction @userAuthorised={{this.userAuthorised}}>
        <:default><div data-test-privileged>privileged</div></:default>
        <:placeholder><div data-test-placeholder>placeholder</div></:placeholder>
        <:unprivileged><div data-test-unprivileged>unprivileged</div></:unprivileged>
      </PrivilegedAction>
    `);
    };
  });

  test('unprivileged block is shown to logged out users', async function (assert) {
    await this.renderComponent(false);
    assert.dom('[data-test-privileged]').doesNotExist();
    assert.dom('[data-test-placeholder]').doesNotExist();
    assert.dom('[data-test-unprivileged]').exists();
  });

  test('unprivileged block is shown to a logged in user without access', async function (assert) {
    const user = this.db.user.create();
    this.authenticateAs(user);

    await this.renderComponent(false);
    assert.dom('[data-test-privileged]').doesNotExist();
    assert.dom('[data-test-placeholder]').doesNotExist();
    assert.dom('[data-test-unprivileged]').exists();
  });

  test('privileged block is shown to a logged in user with access', async function (assert) {
    const user = this.db.user.create();
    this.authenticateAs(user);

    await this.renderComponent(true);
    assert.dom('[data-test-privileged]').exists();
    assert.dom('[data-test-placeholder]').doesNotExist();
    assert.dom('[data-test-unprivileged]').doesNotExist();
  });

  test('placeholder block is shown to a logged in admin without sudo', async function (assert) {
    const user = this.db.user.create({ isAdmin: true });
    this.authenticateAs(user);

    const session = this.owner.lookup('service:session');
    let { currentUser } = await session.loadUserTask.perform();
    assert.true(currentUser.is_admin);
    assert.false(session.isSudoEnabled);

    await this.renderComponent(false);
    assert.dom('[data-test-privileged]').doesNotExist();
    assert.dom('[data-test-placeholder]').exists();
    assert.dom('[data-test-unprivileged]').doesNotExist();
  });

  test('privileged block is shown to a logged in admin without sudo with access', async function (assert) {
    const user = this.db.user.create({ isAdmin: true });
    this.authenticateAs(user);

    const session = this.owner.lookup('service:session');
    let { currentUser } = await session.loadUserTask.perform();
    assert.true(currentUser.is_admin);
    assert.false(session.isSudoEnabled);

    await this.renderComponent(true);
    assert.dom('[data-test-privileged]').exists();
    assert.dom('[data-test-placeholder]').doesNotExist();
    assert.dom('[data-test-unprivileged]').doesNotExist();
  });

  test('privileged block is shown to a logged in admin with sudo', async function (assert) {
    const user = this.db.user.create({ isAdmin: true });
    this.authenticateAs(user);

    const session = this.owner.lookup('service:session');
    let { currentUser } = await session.loadUserTask.perform();
    assert.true(currentUser.is_admin);
    session.setSudo(86_400_000);
    assert.true(session.isSudoEnabled);

    await this.renderComponent(false);
    assert.dom('[data-test-privileged]').exists();
    assert.dom('[data-test-placeholder]').doesNotExist();
    assert.dom('[data-test-unprivileged]').doesNotExist();
  });

  test('automatic placeholder block', async function (assert) {
    const user = this.db.user.create({ isAdmin: true });
    this.authenticateAs(user);

    const session = this.owner.lookup('service:session');
    let { currentUser } = await session.loadUserTask.perform();
    assert.true(currentUser.is_admin);
    assert.false(session.isSudoEnabled);

    // We're trying to confirm that all the form controls are automatically
    // disabled.
    await render(hbs`
      <div data-test-content>
        <PrivilegedAction @userAuthorised={{false}}>
          <button data-test-control type="button">Click me maybe?</button>
          <label for="input">Input: </label><input data-test-control type="text" id="input" />
          <label for="select">Select: </label><select data-test-control id="select"><option>foo</option></select>
          <label for="textarea">Textarea: </label><textarea data-test-control id="textarea" />
        </PrivilegedAction>
      </div>
    `);
    assert.dom('[data-test-content] fieldset').exists().isDisabled();
    assert.dom('[data-test-content] fieldset [data-test-control]').exists();
  });

  test('automatic unprivileged block', async function (assert) {
    // We're testing that the default block content isn't shown, and that the
    // automatically generated div has no content.
    await render(hbs`
      <div data-test-container>
        <PrivilegedAction @userAuthorised={{false}}>
          <div data-test-content>should not be shown</div>
        </PrivilegedAction>
      </div>
    `);
    assert.dom('[data-test-content]').doesNotExist();
    assert.dom('[data-test-container] > div').exists().hasNoText();
  });
});
