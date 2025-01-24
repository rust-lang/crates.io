import { module, test } from 'qunit';

import { http, HttpResponse } from 'msw';

import { setupTest } from 'crates-io/tests/helpers';
import setupMsw from 'crates-io/tests/helpers/setup-msw';
import ajax, { AjaxError, HttpError } from 'crates-io/utils/ajax';

module('ajax()', function (hooks) {
  setupTest(hooks);
  setupMsw(hooks);
  setupFetchRestore(hooks);

  test('fetches a JSON document from the worker', async function (assert) {
    this.worker.use(http.get('/foo', () => HttpResponse.json({ foo: 42 })));

    let response = await ajax('/foo');
    assert.deepEqual(response, { foo: 42 });
  });

  test('passes additional options to `fetch()`', async function (assert) {
    this.worker.use(
      http.get('/foo', () => HttpResponse.json({ foo: 42 })),
      http.put('/foo', () => HttpResponse.json({ foo: 'bar' })),
    );

    let response = await ajax('/foo', { method: 'PUT' });
    assert.deepEqual(response, { foo: 'bar' });
  });

  test('throws an `HttpError` for 5xx responses', async function (assert) {
    this.worker.use(http.get('/foo', () => HttpResponse.json({ foo: 42 }, { status: 500 })));

    await assert.rejects(ajax('/foo'), function (error) {
      let expectedMessage = 'GET /foo failed\n\ncaused by: HttpError: GET /foo failed with: 500 Internal Server Error';

      assert.ok(error instanceof AjaxError);
      assert.strictEqual(error.name, 'AjaxError');
      assert.ok(error.message.startsWith(expectedMessage), error.message);
      assert.strictEqual(error.method, 'GET');
      assert.strictEqual(error.url, '/foo');
      assert.ok(error.cause);
      assert.true(error.isHttpError);
      assert.true(error.isServerError);
      assert.false(error.isClientError);
      assert.false(error.isJsonError);
      assert.false(error.isNetworkError);

      let { cause } = error;
      assert.ok(cause instanceof HttpError);
      assert.strictEqual(cause.name, 'HttpError');
      assert.strictEqual(cause.message, 'GET /foo failed with: 500 Internal Server Error');
      assert.strictEqual(cause.method, 'GET');
      assert.strictEqual(cause.url, '/foo');
      assert.ok(cause.response);
      assert.ok(cause.response.url.endsWith('/foo'));
      return true;
    });
  });

  test('throws an `HttpError` for 4xx responses', async function (assert) {
    this.worker.use(http.get('/foo', () => HttpResponse.json({ foo: 42 }, { status: 404 })));

    await assert.rejects(ajax('/foo'), function (error) {
      let expectedMessage = 'GET /foo failed\n\ncaused by: HttpError: GET /foo failed with: 404 Not Found';

      assert.ok(error instanceof AjaxError);
      assert.strictEqual(error.name, 'AjaxError');
      assert.ok(error.message.startsWith(expectedMessage), error.message);
      assert.strictEqual(error.method, 'GET');
      assert.strictEqual(error.url, '/foo');
      assert.ok(error.cause);
      assert.true(error.isHttpError);
      assert.false(error.isServerError);
      assert.true(error.isClientError);
      assert.false(error.isJsonError);
      assert.false(error.isNetworkError);

      let { cause } = error;
      assert.ok(cause instanceof HttpError);
      assert.strictEqual(cause.name, 'HttpError');
      assert.strictEqual(cause.message, 'GET /foo failed with: 404 Not Found');
      assert.strictEqual(cause.method, 'GET');
      assert.strictEqual(cause.url, '/foo');
      assert.ok(cause.response);
      assert.ok(cause.response.url.endsWith('/foo'));
      return true;
    });
  });

  test('throws an error for invalid JSON responses', async function (assert) {
    this.worker.use(http.get('/foo', () => HttpResponse.text('{ foo: 42')));

    await assert.rejects(ajax('/foo'), function (error) {
      let expectedMessage = 'GET /foo failed\n\ncaused by: SyntaxError';

      assert.ok(error instanceof AjaxError);
      assert.strictEqual(error.name, 'AjaxError');
      assert.ok(error.message.startsWith(expectedMessage), error.message);
      assert.strictEqual(error.method, 'GET');
      assert.strictEqual(error.url, '/foo');
      assert.ok(error.cause);
      assert.false(error.isHttpError);
      assert.false(error.isServerError);
      assert.false(error.isClientError);
      assert.true(error.isJsonError);
      assert.false(error.isNetworkError);

      let expectedCauseMessages = [
        // Chrome < 104
        'Unexpected token f in JSON at position 2',
        // Chrome 104 – 117
        "Expected property name or '}' in JSON at position 2",
        // Chrome >= 117
        "Expected property name or '}' in JSON at position 2 (line 1 column 3)",
      ];

      let { cause } = error;
      assert.notOk(cause instanceof HttpError);
      assert.strictEqual(cause.name, 'SyntaxError');
      assert.ok(expectedCauseMessages.includes(cause.message), `"${cause.message}" is an expected error message`);
      return true;
    });
  });

  test('throws an error when there is a network issue', async function (assert) {
    window.fetch = async function () {
      throw new TypeError('NetworkError when attempting to fetch resource.');
    };

    await assert.rejects(ajax('/foo'), function (error) {
      let expectedMessage = 'GET /foo failed\n\ncaused by: TypeError';

      assert.ok(error instanceof AjaxError);
      assert.strictEqual(error.name, 'AjaxError');
      assert.ok(error.message.startsWith(expectedMessage), error.message);
      assert.strictEqual(error.method, 'GET');
      assert.strictEqual(error.url, '/foo');
      assert.ok(error.cause);
      assert.false(error.isHttpError);
      assert.false(error.isServerError);
      assert.false(error.isClientError);
      assert.false(error.isJsonError);
      assert.true(error.isNetworkError);

      let { cause } = error;
      assert.notOk(cause instanceof HttpError);
      assert.strictEqual(cause.name, 'TypeError');
      assert.strictEqual(cause.message, 'NetworkError when attempting to fetch resource.');
      return true;
    });
  });

  module('json()', function () {
    test('resolves with the JSON payload', async function (assert) {
      this.worker.use(http.get('/foo', () => HttpResponse.json({ foo: 42 }, { status: 500 })));

      let error;
      await assert.rejects(ajax('/foo'), function (_error) {
        error = _error;
        return true;
      });

      let json = await error.json();
      assert.deepEqual(json, { foo: 42 });
    });

    test('resolves with `undefined` if there is no JSON payload', async function (assert) {
      this.worker.use(http.get('/foo', () => HttpResponse.text('{ foo: 42', { status: 500 })));

      let error;
      await assert.rejects(ajax('/foo'), function (_error) {
        error = _error;
        return true;
      });

      let json = await error.json();
      assert.strictEqual(json, undefined);
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
