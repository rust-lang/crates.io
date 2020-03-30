import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { visit } from '@ember/test-helpers';
import { Response } from 'ember-cli-mirage';
import setupMirage from '../helpers/setup-mirage';
import window, { setupWindowMock } from 'ember-window-mock';

module('Route | github-authorized', function (hooks) {
  setupApplicationTest(hooks);
  setupWindowMock(hooks);
  setupMirage(hooks);

  test('happy path', async function (assert) {
    assert.expect(4);

    window.close = () => assert.step('window.close()');
    window.opener = {};

    this.server.get('/api/private/session/authorize', (schema, request) => {
      assert.deepEqual(request.queryParams, {
        code: '901dd10e07c7e9fa1cd5',
        state: 'fYcUY3FMdUUz00FC7vLT7A',
      });

      return {
        user: {
          id: 42,
          login: 'johnnydee',
          email_verified: true,
          email_verification_sent: true,
          name: 'John Doe',
          email: 'john@doe.com',
          avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
          url: 'https://github.com/johnnydee',
        },
      };
    });

    await visit('/authorize/github?code=901dd10e07c7e9fa1cd5&state=fYcUY3FMdUUz00FC7vLT7A');

    assert.deepEqual(JSON.parse(window.opener.github_response), {
      data: {
        user: {
          id: 42,
          avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
          email: 'john@doe.com',
          email_verification_sent: true,
          email_verified: true,
          login: 'johnnydee',
          name: 'John Doe',
          url: 'https://github.com/johnnydee',
        },
      },
      ok: true,
    });

    assert.verifySteps(['window.close()']);
  });

  test('sad path', async function (assert) {
    assert.expect(3);

    window.close = () => assert.step('window.close()');
    window.opener = {};

    this.server.get('/api/private/session/authorize', () => new Response(500));

    await visit('/authorize/github?code=901dd10e07c7e9fa1cd5&state=fYcUY3FMdUUz00FC7vLT7A');

    assert.strictEqual(JSON.parse(window.opener.github_response).ok, false);
    assert.verifySteps(['window.close()']);
  });
});
