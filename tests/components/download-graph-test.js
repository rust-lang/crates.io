import { click, render, settled, waitFor } from '@ember/test-helpers';
import { module, test } from 'qunit';

import Service from '@ember/service';
import { defer } from 'rsvp';

import { hbs } from 'ember-cli-htmlbars';
import { dropTask } from 'ember-concurrency';
import window from 'ember-window-mock';
import { setupWindowMock } from 'ember-window-mock/test-support';

import { setupRenderingTest } from 'cargo/tests/helpers';

import { toChartData } from '../../components/download-graph';
import ChartJsLoader from '../../services/chartjs';
import { setupFakeTimers } from '../helpers/fake-timers';

module('Component | DownloadGraph', function (hooks) {
  setupRenderingTest(hooks);
  setupWindowMock(hooks);
  setupFakeTimers(hooks, '2020-12-30T12:34:56Z');

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

    class MockService extends ChartJsLoader {
      @dropTask *loadTask() {
        yield deferred.promise;
        return yield super.loadTask.perform();
      }
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
      @dropTask *loadTask() {
        throw new Error('nope');
      }
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

  module('toChartData()', function () {
    test('converts raw download data to Chart.js format', function (assert) {
      let data = exampleData();
      let result = toChartData(data);
      assert.matchJson(result, {
        datasets: [
          {
            backgroundColor: '#d3b5bc',
            borderColor: '#67001f',
            borderWidth: 2,
            cubicInterpolationMode: 'monotone',
            data: [
              { x: isDate('2020-12-30'), y: 30_520 },
              { x: isDate('2020-12-29'), y: 31_631 },
              { x: isDate('2020-12-28'), y: 0 },
            ],
            fill: 'origin',
            label: '1.0.56',
            pointHoverBorderWidth: 2,
            pointHoverRadius: 5,
          },
          {
            backgroundColor: '#eabdc0',
            borderColor: '#b2182b',
            borderWidth: 2,
            cubicInterpolationMode: 'monotone',
            data: [
              { x: isDate('2020-12-30'), y: 3702 },
              { x: isDate('2020-12-29'), y: 4157 },
              { x: isDate('2020-12-28'), y: 2414 },
              { x: isDate('2020-12-27'), y: 15_713 },
              { x: isDate('2020-12-26'), y: 0 },
            ],
            fill: 'origin',
            label: '1.0.55',
            pointHoverBorderWidth: 2,
            pointHoverRadius: 5,
          },
          {
            backgroundColor: '#f3d0ca',
            borderColor: '#d6604d',
            borderWidth: 2,
            cubicInterpolationMode: 'monotone',
            data: [
              { x: isDate('2020-12-30'), y: 4298 },
              { x: isDate('2020-12-29'), y: 4277 },
              { x: isDate('2020-12-28'), y: 2786 },
              { x: isDate('2020-12-27'), y: 2477 },
              { x: isDate('2020-12-26'), y: 0 },
              { x: isDate('2020-12-25'), y: 0 },
              { x: isDate('2020-12-24'), y: 0 },
              { x: isDate('2020-12-23'), y: 0 },
              { x: isDate('2020-12-22'), y: 0 },
              { x: isDate('2020-12-21'), y: 0 },
              { x: isDate('2020-12-20'), y: 0 },
              { x: isDate('2020-12-19'), y: 0 },
              { x: isDate('2020-12-18'), y: 0 },
              { x: isDate('2020-12-17'), y: 0 },
              { x: isDate('2020-12-16'), y: 0 },
              { x: isDate('2020-12-15'), y: 0 },
              { x: isDate('2020-12-14'), y: 0 },
              { x: isDate('2020-12-13'), y: 0 },
              { x: isDate('2020-12-12'), y: 0 },
              { x: isDate('2020-12-11'), y: 0 },
              { x: isDate('2020-12-10'), y: 0 },
              { x: isDate('2020-12-09'), y: 0 },
              { x: isDate('2020-12-08'), y: 0 },
              { x: isDate('2020-12-07'), y: 0 },
              { x: isDate('2020-12-06'), y: 0 },
              { x: isDate('2020-12-05'), y: 0 },
              { x: isDate('2020-12-04'), y: 0 },
              { x: isDate('2020-12-03'), y: 0 },
              { x: isDate('2020-12-02'), y: 0 },
              { x: isDate('2020-12-01'), y: 0 },
              { x: isDate('2020-11-30'), y: 0 },
              { x: isDate('2020-11-29'), y: 0 },
              { x: isDate('2020-11-28'), y: 0 },
              { x: isDate('2020-11-27'), y: 0 },
              { x: isDate('2020-11-26'), y: 0 },
              { x: isDate('2020-11-25'), y: 0 },
              { x: isDate('2020-11-24'), y: 0 },
              { x: isDate('2020-11-23'), y: 0 },
              { x: isDate('2020-11-22'), y: 0 },
              { x: isDate('2020-11-21'), y: 0 },
              { x: isDate('2020-11-20'), y: 0 },
              { x: isDate('2020-11-19'), y: 0 },
              { x: isDate('2020-11-18'), y: 0 },
              { x: isDate('2020-11-17'), y: 0 },
              { x: isDate('2020-11-16'), y: 0 },
              { x: isDate('2020-11-15'), y: 0 },
              { x: isDate('2020-11-14'), y: 0 },
            ],
            fill: 'origin',
            label: '1.0.54',
            pointHoverBorderWidth: 2,
            pointHoverRadius: 5,
          },
          {
            backgroundColor: '#fce4d9',
            borderColor: '#f4a582',
            borderWidth: 2,
            cubicInterpolationMode: 'monotone',
            data: [
              { x: isDate('2020-12-30'), y: 2228 },
              { x: isDate('2020-12-29'), y: 1650 },
              { x: isDate('2020-12-28'), y: 968 },
              { x: isDate('2020-12-27'), y: 873 },
              { x: isDate('2020-12-26'), y: 0 },
              { x: isDate('2020-12-25'), y: 0 },
              { x: isDate('2020-12-24'), y: 0 },
              { x: isDate('2020-12-23'), y: 0 },
              { x: isDate('2020-12-22'), y: 0 },
              { x: isDate('2020-12-21'), y: 0 },
              { x: isDate('2020-12-20'), y: 0 },
              { x: isDate('2020-12-19'), y: 0 },
              { x: isDate('2020-12-18'), y: 0 },
              { x: isDate('2020-12-17'), y: 0 },
              { x: isDate('2020-12-16'), y: 0 },
              { x: isDate('2020-12-15'), y: 0 },
              { x: isDate('2020-12-14'), y: 0 },
              { x: isDate('2020-12-13'), y: 0 },
              { x: isDate('2020-12-12'), y: 0 },
              { x: isDate('2020-12-11'), y: 0 },
              { x: isDate('2020-12-10'), y: 0 },
              { x: isDate('2020-12-09'), y: 0 },
              { x: isDate('2020-12-08'), y: 0 },
              { x: isDate('2020-12-07'), y: 0 },
              { x: isDate('2020-12-06'), y: 0 },
              { x: isDate('2020-12-05'), y: 0 },
              { x: isDate('2020-12-04'), y: 0 },
              { x: isDate('2020-12-03'), y: 0 },
              { x: isDate('2020-12-02'), y: 0 },
              { x: isDate('2020-12-01'), y: 0 },
              { x: isDate('2020-11-30'), y: 0 },
              { x: isDate('2020-11-29'), y: 0 },
              { x: isDate('2020-11-28'), y: 0 },
              { x: isDate('2020-11-27'), y: 0 },
              { x: isDate('2020-11-26'), y: 0 },
              { x: isDate('2020-11-25'), y: 0 },
              { x: isDate('2020-11-24'), y: 0 },
              { x: isDate('2020-11-23'), y: 0 },
              { x: isDate('2020-11-22'), y: 0 },
              { x: isDate('2020-11-21'), y: 0 },
              { x: isDate('2020-11-20'), y: 0 },
              { x: isDate('2020-11-19'), y: 0 },
              { x: isDate('2020-11-18'), y: 0 },
              { x: isDate('2020-11-17'), y: 0 },
              { x: isDate('2020-11-16'), y: 0 },
              { x: isDate('2020-11-15'), y: 0 },
              { x: isDate('2020-11-14'), y: 0 },
              { x: isDate('2020-11-13'), y: 0 },
              { x: isDate('2020-11-12'), y: 0 },
              { x: isDate('2020-11-11'), y: 0 },
              { x: isDate('2020-11-10'), y: 0 },
              { x: isDate('2020-11-09'), y: 0 },
              { x: isDate('2020-11-08'), y: 0 },
              { x: isDate('2020-11-07'), y: 0 },
              { x: isDate('2020-11-06'), y: 0 },
              { x: isDate('2020-11-05'), y: 0 },
              { x: isDate('2020-11-04'), y: 0 },
              { x: isDate('2020-11-03'), y: 0 },
              { x: isDate('2020-11-02'), y: 0 },
              { x: isDate('2020-11-01'), y: 0 },
              { x: isDate('2020-10-31'), y: 0 },
              { x: isDate('2020-10-30'), y: 0 },
              { x: isDate('2020-10-29'), y: 0 },
              { x: isDate('2020-10-28'), y: 0 },
              { x: isDate('2020-10-27'), y: 0 },
              { x: isDate('2020-10-26'), y: 0 },
              { x: isDate('2020-10-25'), y: 0 },
              { x: isDate('2020-10-24'), y: 0 },
              { x: isDate('2020-10-23'), y: 0 },
              { x: isDate('2020-10-22'), y: 0 },
              { x: isDate('2020-10-21'), y: 0 },
              { x: isDate('2020-10-20'), y: 0 },
              { x: isDate('2020-10-19'), y: 0 },
              { x: isDate('2020-10-18'), y: 0 },
              { x: isDate('2020-10-17'), y: 0 },
              { x: isDate('2020-10-16'), y: 0 },
              { x: isDate('2020-10-15'), y: 0 },
              { x: isDate('2020-10-14'), y: 0 },
              { x: isDate('2020-10-13'), y: 0 },
              { x: isDate('2020-10-12'), y: 0 },
              { x: isDate('2020-10-11'), y: 0 },
              { x: isDate('2020-10-10'), y: 0 },
              { x: isDate('2020-10-09'), y: 0 },
              { x: isDate('2020-10-08'), y: 0 },
              { x: isDate('2020-10-07'), y: 0 },
              { x: isDate('2020-10-06'), y: 0 },
              { x: isDate('2020-10-05'), y: 0 },
              { x: isDate('2020-10-04'), y: 0 },
            ],
            fill: 'origin',
            label: '1.0.53',
            pointHoverBorderWidth: 2,
            pointHoverRadius: 5,
          },
          {
            backgroundColor: '#deedf5',
            borderColor: '#92c5de',
            borderWidth: 2,
            cubicInterpolationMode: 'monotone',
            data: [
              { x: isDate('2020-12-30'), y: 201 },
              { x: isDate('2020-12-29'), y: 261 },
              { x: isDate('2020-12-28'), y: 181 },
              { x: isDate('2020-12-27'), y: 186 },
              { x: isDate('2020-12-26'), y: 0 },
              { x: isDate('2020-12-25'), y: 0 },
              { x: isDate('2020-12-24'), y: 0 },
              { x: isDate('2020-12-23'), y: 0 },
              { x: isDate('2020-12-22'), y: 0 },
              { x: isDate('2020-12-21'), y: 0 },
              { x: isDate('2020-12-20'), y: 0 },
              { x: isDate('2020-12-19'), y: 0 },
              { x: isDate('2020-12-18'), y: 0 },
              { x: isDate('2020-12-17'), y: 0 },
              { x: isDate('2020-12-16'), y: 0 },
              { x: isDate('2020-12-15'), y: 0 },
              { x: isDate('2020-12-14'), y: 0 },
              { x: isDate('2020-12-13'), y: 0 },
              { x: isDate('2020-12-12'), y: 0 },
              { x: isDate('2020-12-11'), y: 0 },
              { x: isDate('2020-12-10'), y: 0 },
              { x: isDate('2020-12-09'), y: 0 },
              { x: isDate('2020-12-08'), y: 0 },
              { x: isDate('2020-12-07'), y: 0 },
              { x: isDate('2020-12-06'), y: 0 },
              { x: isDate('2020-12-05'), y: 0 },
              { x: isDate('2020-12-04'), y: 0 },
              { x: isDate('2020-12-03'), y: 0 },
              { x: isDate('2020-12-02'), y: 0 },
              { x: isDate('2020-12-01'), y: 0 },
              { x: isDate('2020-11-30'), y: 0 },
              { x: isDate('2020-11-29'), y: 0 },
              { x: isDate('2020-11-28'), y: 0 },
              { x: isDate('2020-11-27'), y: 0 },
              { x: isDate('2020-11-26'), y: 0 },
              { x: isDate('2020-11-25'), y: 0 },
              { x: isDate('2020-11-24'), y: 0 },
              { x: isDate('2020-11-23'), y: 0 },
              { x: isDate('2020-11-22'), y: 0 },
              { x: isDate('2020-11-21'), y: 0 },
              { x: isDate('2020-11-20'), y: 0 },
              { x: isDate('2020-11-19'), y: 0 },
              { x: isDate('2020-11-18'), y: 0 },
              { x: isDate('2020-11-17'), y: 0 },
              { x: isDate('2020-11-16'), y: 0 },
              { x: isDate('2020-11-15'), y: 0 },
              { x: isDate('2020-11-14'), y: 0 },
              { x: isDate('2020-11-13'), y: 0 },
              { x: isDate('2020-11-12'), y: 0 },
              { x: isDate('2020-11-11'), y: 0 },
              { x: isDate('2020-11-10'), y: 0 },
              { x: isDate('2020-11-09'), y: 0 },
              { x: isDate('2020-11-08'), y: 0 },
              { x: isDate('2020-11-07'), y: 0 },
              { x: isDate('2020-11-06'), y: 0 },
              { x: isDate('2020-11-05'), y: 0 },
              { x: isDate('2020-11-04'), y: 0 },
              { x: isDate('2020-11-03'), y: 0 },
              { x: isDate('2020-11-02'), y: 0 },
              { x: isDate('2020-11-01'), y: 0 },
              { x: isDate('2020-10-31'), y: 0 },
              { x: isDate('2020-10-30'), y: 0 },
              { x: isDate('2020-10-29'), y: 0 },
              { x: isDate('2020-10-28'), y: 0 },
              { x: isDate('2020-10-27'), y: 0 },
              { x: isDate('2020-10-26'), y: 0 },
              { x: isDate('2020-10-25'), y: 0 },
              { x: isDate('2020-10-24'), y: 0 },
              { x: isDate('2020-10-23'), y: 0 },
              { x: isDate('2020-10-22'), y: 0 },
              { x: isDate('2020-10-21'), y: 0 },
              { x: isDate('2020-10-20'), y: 0 },
              { x: isDate('2020-10-19'), y: 0 },
              { x: isDate('2020-10-18'), y: 0 },
              { x: isDate('2020-10-17'), y: 0 },
              { x: isDate('2020-10-16'), y: 0 },
              { x: isDate('2020-10-15'), y: 0 },
              { x: isDate('2020-10-14'), y: 0 },
              { x: isDate('2020-10-13'), y: 0 },
              { x: isDate('2020-10-12'), y: 0 },
              { x: isDate('2020-10-11'), y: 0 },
              { x: isDate('2020-10-10'), y: 0 },
              { x: isDate('2020-10-09'), y: 0 },
              { x: isDate('2020-10-08'), y: 0 },
              { x: isDate('2020-10-07'), y: 0 },
              { x: isDate('2020-10-06'), y: 0 },
              { x: isDate('2020-10-05'), y: 0 },
              { x: isDate('2020-10-04'), y: 0 },
              { x: isDate('2020-10-03'), y: 0 },
              { x: isDate('2020-10-02'), y: 0 },
            ],
            fill: 'origin',
            label: '1.0.52',
            pointHoverBorderWidth: 2,
            pointHoverRadius: 5,
          },
          {
            backgroundColor: '#c9deed',
            borderColor: '#4393c3',
            borderWidth: 2,
            cubicInterpolationMode: 'monotone',
            data: [
              { x: isDate('2020-12-30'), y: 36_745 },
              { x: isDate('2020-12-29'), y: 33_242 },
              { x: isDate('2020-12-28'), y: 19_981 },
              { x: isDate('2020-12-27'), y: 19_064 },
              { x: isDate('2020-12-26'), y: 0 },
              { x: isDate('2020-12-25'), y: 0 },
              { x: isDate('2020-12-24'), y: 0 },
              { x: isDate('2020-12-23'), y: 0 },
              { x: isDate('2020-12-22'), y: 0 },
              { x: isDate('2020-12-21'), y: 0 },
              { x: isDate('2020-12-20'), y: 0 },
              { x: isDate('2020-12-19'), y: 0 },
              { x: isDate('2020-12-18'), y: 0 },
              { x: isDate('2020-12-17'), y: 0 },
              { x: isDate('2020-12-16'), y: 0 },
              { x: isDate('2020-12-15'), y: 0 },
              { x: isDate('2020-12-14'), y: 0 },
              { x: isDate('2020-12-13'), y: 0 },
              { x: isDate('2020-12-12'), y: 0 },
              { x: isDate('2020-12-11'), y: 0 },
              { x: isDate('2020-12-10'), y: 0 },
              { x: isDate('2020-12-09'), y: 0 },
              { x: isDate('2020-12-08'), y: 0 },
              { x: isDate('2020-12-07'), y: 0 },
              { x: isDate('2020-12-06'), y: 0 },
              { x: isDate('2020-12-05'), y: 0 },
              { x: isDate('2020-12-04'), y: 0 },
              { x: isDate('2020-12-03'), y: 0 },
              { x: isDate('2020-12-02'), y: 0 },
              { x: isDate('2020-12-01'), y: 0 },
              { x: isDate('2020-11-30'), y: 0 },
              { x: isDate('2020-11-29'), y: 0 },
              { x: isDate('2020-11-28'), y: 0 },
              { x: isDate('2020-11-27'), y: 0 },
              { x: isDate('2020-11-26'), y: 0 },
              { x: isDate('2020-11-25'), y: 0 },
              { x: isDate('2020-11-24'), y: 0 },
              { x: isDate('2020-11-23'), y: 0 },
              { x: isDate('2020-11-22'), y: 0 },
              { x: isDate('2020-11-21'), y: 0 },
              { x: isDate('2020-11-20'), y: 0 },
              { x: isDate('2020-11-19'), y: 0 },
              { x: isDate('2020-11-18'), y: 0 },
              { x: isDate('2020-11-17'), y: 0 },
              { x: isDate('2020-11-16'), y: 0 },
              { x: isDate('2020-11-15'), y: 0 },
              { x: isDate('2020-11-14'), y: 0 },
              { x: isDate('2020-11-13'), y: 0 },
              { x: isDate('2020-11-12'), y: 0 },
              { x: isDate('2020-11-11'), y: 0 },
              { x: isDate('2020-11-10'), y: 0 },
              { x: isDate('2020-11-09'), y: 0 },
              { x: isDate('2020-11-08'), y: 0 },
              { x: isDate('2020-11-07'), y: 0 },
              { x: isDate('2020-11-06'), y: 0 },
              { x: isDate('2020-11-05'), y: 0 },
              { x: isDate('2020-11-04'), y: 0 },
              { x: isDate('2020-11-03'), y: 0 },
              { x: isDate('2020-11-02'), y: 0 },
              { x: isDate('2020-11-01'), y: 0 },
              { x: isDate('2020-10-31'), y: 0 },
              { x: isDate('2020-10-30'), y: 0 },
              { x: isDate('2020-10-29'), y: 0 },
              { x: isDate('2020-10-28'), y: 0 },
              { x: isDate('2020-10-27'), y: 0 },
              { x: isDate('2020-10-26'), y: 0 },
              { x: isDate('2020-10-25'), y: 0 },
              { x: isDate('2020-10-24'), y: 0 },
              { x: isDate('2020-10-23'), y: 0 },
              { x: isDate('2020-10-22'), y: 0 },
              { x: isDate('2020-10-21'), y: 0 },
              { x: isDate('2020-10-20'), y: 0 },
              { x: isDate('2020-10-19'), y: 0 },
              { x: isDate('2020-10-18'), y: 0 },
              { x: isDate('2020-10-17'), y: 0 },
              { x: isDate('2020-10-16'), y: 0 },
              { x: isDate('2020-10-15'), y: 0 },
              { x: isDate('2020-10-14'), y: 0 },
              { x: isDate('2020-10-13'), y: 0 },
              { x: isDate('2020-10-12'), y: 0 },
              { x: isDate('2020-10-11'), y: 0 },
              { x: isDate('2020-10-10'), y: 0 },
              { x: isDate('2020-10-09'), y: 0 },
              { x: isDate('2020-10-08'), y: 0 },
              { x: isDate('2020-10-07'), y: 0 },
              { x: isDate('2020-10-06'), y: 0 },
              { x: isDate('2020-10-05'), y: 0 },
              { x: isDate('2020-10-04'), y: 0 },
              { x: isDate('2020-10-03'), y: 0 },
              { x: isDate('2020-10-02'), y: 0 },
            ],
            fill: 'origin',
            label: 'Other',
            pointHoverBorderWidth: 2,
            pointHoverRadius: 5,
          },
        ],
      });
    });
  });
});

function exampleData() {
  let FIVE_2 = { num: '1.0.52', created_at: new Date('2020-10-01') };
  let FIVE_3 = { num: '1.0.53', created_at: new Date('2020-10-05') };
  let FIVE_4 = { num: '1.0.54', created_at: new Date('2020-11-15') };
  let FIVE_5 = { num: '1.0.55', created_at: new Date('2020-12-27') };
  let FIVE_6 = { num: '1.0.56', created_at: new Date('2020-12-29') };

  let downloads = [
    { version: FIVE_2, date: '2020-12-30', downloads: 201 },
    { version: FIVE_3, date: '2020-12-30', downloads: 2228 },
    { version: FIVE_4, date: '2020-12-30', downloads: 4298 },
    { version: FIVE_5, date: '2020-12-30', downloads: 3702 },
    { version: FIVE_6, date: '2020-12-30', downloads: 30_520 },
    { version: FIVE_2, date: '2020-12-29', downloads: 261 },
    { version: FIVE_3, date: '2020-12-29', downloads: 1650 },
    { version: FIVE_4, date: '2020-12-29', downloads: 4277 },
    { version: FIVE_5, date: '2020-12-29', downloads: 4157 },
    { version: FIVE_6, date: '2020-12-29', downloads: 31_631 },
    { version: FIVE_2, date: '2020-12-28', downloads: 181 },
    { version: FIVE_3, date: '2020-12-28', downloads: 968 },
    { version: FIVE_4, date: '2020-12-28', downloads: 2786 },
    { version: FIVE_5, date: '2020-12-28', downloads: 2414 },
    { version: FIVE_2, date: '2020-12-27', downloads: 186 },
    { version: FIVE_3, date: '2020-12-27', downloads: 873 },
    { version: FIVE_4, date: '2020-12-27', downloads: 2477 },
    { version: FIVE_5, date: '2020-12-27', downloads: 15_713 },
  ];

  downloads.meta = {
    extra_downloads: [
      { date: '2020-12-30', downloads: 36_745 },
      { date: '2020-12-29', downloads: 33_242 },
      { date: '2020-12-28', downloads: 19_981 },
      { date: '2020-12-27', downloads: 19_064 },
    ],
  };

  return downloads;
}

function isDate(isoDate) {
  return function (date) {
    return date.toISOString().startsWith(isoDate);
  };
}
