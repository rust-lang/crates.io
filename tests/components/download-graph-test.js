import { click, render, settled, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import Service from '@ember/service';
import { defer } from 'rsvp';

import { hbs } from 'ember-cli-htmlbars';
import { task } from 'ember-concurrency';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupRenderingTest } from 'cargo/tests/helpers';

module('Component | DownloadGraph', function (hooks) {
  setupRenderingTest(hooks);
  setupWindowMock(hooks);

  test('happy path', async function (assert) {
    this.data = exampleData();

    await render(hbs`<DownloadGraph @data={{this.data}} />`);
    assert.dom('[data-test-download-graph]').exists();
    assert.dom('[data-test-download-graph] [data-test-spinner]').doesNotExist();
    assert.dom('[data-test-download-graph] canvas').exists();
    assert.dom('[data-test-download-graph] [data-test-error]').doesNotExist();
  });

  test('loading spinner', async function (assert) {
    this.data = exampleData();

    let deferred = defer();

    class MockService extends Service {
      @(task(function* () {
        yield deferred.promise;
        return yield import('chart.js').then(module => module.default);
      }).drop())
      loadTask;
    }

    this.owner.register('service:chartjs', MockService);

    render(hbs`<DownloadGraph @data={{this.data}} />`);
    await waitFor('[data-test-download-graph] [data-test-spinner]');
    assert.dom('[data-test-download-graph]').exists();
    assert.dom('[data-test-download-graph] [data-test-spinner]').exists();
    assert.dom('[data-test-download-graph] canvas').doesNotExist();
    assert.dom('[data-test-download-graph] [data-test-error]').doesNotExist();

    deferred.resolve();
    await settled();
    assert.dom('[data-test-download-graph]').exists();
    assert.dom('[data-test-download-graph] [data-test-spinner]').doesNotExist();
    assert.dom('[data-test-download-graph] canvas').exists();
    assert.dom('[data-test-download-graph] [data-test-error]').doesNotExist();
  });

  test('error behavior', async function (assert) {
    class MockService extends Service {
      // eslint-disable-next-line require-yield
      @(task(function* () {
        throw new Error('nope');
      }).drop())
      loadTask;
    }

    this.owner.register('service:chartjs', MockService);

    await render(hbs`<DownloadGraph @data={{this.data}} />`);
    assert.dom('[data-test-download-graph]').exists();
    assert.dom('[data-test-download-graph] [data-test-spinner]').doesNotExist();
    assert.dom('[data-test-download-graph] canvas').doesNotExist();
    assert.dom('[data-test-download-graph] [data-test-error]').exists();

    window.location.reload = () => assert.step('reload');
    await click('[data-test-download-graph] [data-test-reload]');
    assert.verifySteps(['reload']);
  });
});

function exampleData() {
  let downloads = [
    { version: { num: '1.0.52' }, date: '2020-12-30', downloads: 201 },
    { version: { num: '1.0.53' }, date: '2020-12-30', downloads: 2228 },
    { version: { num: '1.0.54' }, date: '2020-12-30', downloads: 4298 },
    { version: { num: '1.0.55' }, date: '2020-12-30', downloads: 3702 },
    { version: { num: '1.0.56' }, date: '2020-12-30', downloads: 30520 },
    { version: { num: '1.0.52' }, date: '2020-12-29', downloads: 261 },
    { version: { num: '1.0.53' }, date: '2020-12-29', downloads: 1650 },
    { version: { num: '1.0.54' }, date: '2020-12-29', downloads: 4277 },
    { version: { num: '1.0.55' }, date: '2020-12-29', downloads: 4157 },
    { version: { num: '1.0.56' }, date: '2020-12-29', downloads: 31631 },
    { version: { num: '1.0.52' }, date: '2020-12-28', downloads: 181 },
    { version: { num: '1.0.53' }, date: '2020-12-28', downloads: 968 },
    { version: { num: '1.0.54' }, date: '2020-12-28', downloads: 2786 },
    { version: { num: '1.0.55' }, date: '2020-12-28', downloads: 2414 },
    { version: { num: '1.0.56' }, date: '2020-12-28', downloads: 23616 },
    { version: { num: '1.0.52' }, date: '2020-12-27', downloads: 186 },
    { version: { num: '1.0.53' }, date: '2020-12-27', downloads: 873 },
    { version: { num: '1.0.54' }, date: '2020-12-27', downloads: 2477 },
    { version: { num: '1.0.55' }, date: '2020-12-27', downloads: 15713 },
    { version: { num: '1.0.56' }, date: '2020-12-27', downloads: 3815 },
  ];

  downloads.content = {
    meta: {
      extra_downloads: [
        { date: '2020-12-30', downloads: 36745 },
        { date: '2020-12-29', downloads: 33242 },
        { date: '2020-12-28', downloads: 19981 },
        { date: '2020-12-27', downloads: 19064 },
      ],
    },
  };

  return downloads;
}
