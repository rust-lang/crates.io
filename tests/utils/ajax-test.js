import { setupTest } from 'ember-qunit';
import { module, test } from 'qunit';

import ajax, { AjaxError, HttpError } from 'cargo/utils/ajax';

import setupMirage from '../helpers/setup-mirage';

module('ajax()', function (hooks) {
  setupTest(hooks);
  setupMirage(hooks);
  setupFetchRestore(hooks);

  test('fetches a JSON document from the server', async function (assert) {
    this.server.get('/foo', { foo: 42 });

    let response = await ajax('/foo');
    assert.deepEqual(response, { foo: 42 });
  });

  test('passes additional options to `fetch()`', async function (assert) {
    this.server.get('/foo', { foo: 42 });
    this.server.put('/foo', { foo: 'bar' });

    let response = await ajax('/foo', { method: 'PUT' });
    assert.deepEqual(response, { foo: 'bar' });
  });

  test('throws an `HttpError` for non-2xx responses', async function (assert) {
    this.server.get('/foo', { foo: 42 }, 500);

    await assert.rejects(ajax('/foo'), function (error) {
      assert.ok(error instanceof AjaxError);
      assert.equal(error.name, 'AjaxError');
      assert.equal(error.message, 'GET /foo failed');
      assert.equal(error.method, 'GET');
      assert.equal(error.url, '/foo');
      assert.ok(error.cause);

      let { cause } = error;
      assert.ok(cause instanceof HttpError);
      assert.equal(cause.name, 'HttpError');
      assert.equal(cause.message, 'GET /foo failed with: 500 Internal Server Error');
      assert.equal(cause.method, 'GET');
      assert.equal(cause.url, '/foo');
      assert.ok(cause.response);
      assert.equal(cause.response.url, '/foo');
      return true;
    });
  });

  test('throws an error for invalid JSON responses', async function (assert) {
    this.server.get('/foo', () => '{ foo: 42');

    await assert.rejects(ajax('/foo'), function (error) {
      assert.ok(error instanceof AjaxError);
      assert.equal(error.name, 'AjaxError');
      assert.equal(error.message, 'GET /foo failed');
      assert.equal(error.method, 'GET');
      assert.equal(error.url, '/foo');
      assert.ok(error.cause);

      let { cause } = error;
      assert.ok(!(cause instanceof HttpError));
      assert.equal(cause.name, 'SyntaxError');
      assert.equal(cause.message, 'Unexpected token f in JSON at position 2');
      return true;
    });
  });

  test('throws an error when there is a network issue', async function (assert) {
    window.fetch = async function () {
      throw new TypeError('NetworkError when attempting to fetch resource.');
    };

    await assert.rejects(ajax('/foo'), function (error) {
      assert.ok(error instanceof AjaxError);
      assert.equal(error.name, 'AjaxError');
      assert.equal(error.message, 'GET /foo failed');
      assert.equal(error.method, 'GET');
      assert.equal(error.url, '/foo');
      assert.ok(error.cause);

      let { cause } = error;
      assert.ok(!(cause instanceof HttpError));
      assert.equal(cause.name, 'TypeError');
      assert.equal(cause.message, 'NetworkError when attempting to fetch resource.');
      return true;
    });
  });
});

function setupFetchRestore(hooks) {
  let oldFetch;
  hooks.beforeEach(function () {
    oldFetch = window.fetch;
  });
  hooks.afterEach(function () {
    window.fetch = oldFetch;
  });
}
