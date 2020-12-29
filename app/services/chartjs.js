import Service from '@ember/service';

import { task } from 'ember-concurrency';

export default class ChartJsLoader extends Service {
  @(task(function* () {
    let Chart = yield import('chart.js').then(module => module.default);
    Chart.platform.disableCSSInjection = true;
    return Chart;
  }).drop())
  loadTask;
}
