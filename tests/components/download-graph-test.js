import { click, render, settled, waitFor } from '@ember/test-helpers';
import { setupRenderingTest } from 'ember-qunit';
import { module, test } from 'qunit';

import Service from '@ember/service';
import { defer } from 'rsvp';

import { hbs } from 'ember-cli-htmlbars';
import { task } from 'ember-concurrency';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

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
  return [
    ['Date', 'Other', '1.0.52', '1.0.53', '1.0.54', '1.0.55', '1.0.56'],
    [new Date('2020-12-30'), 36745, 201, 2228, 4298, 3702, 30520],
    [new Date('2020-12-29'), 33242, 261, 1650, 4277, 4157, 31631],
    [new Date('2020-12-28'), 19981, 181, 968, 2786, 2414, 23616],
    [new Date('2020-12-27'), 19064, 186, 873, 2477, 15713, 3815],
  ];
}
