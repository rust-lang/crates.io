import { alias, bool } from '@ember/object/computed';
import Service from '@ember/service';

import { task } from 'ember-concurrency';

export default class GoogleChartsService extends Service {
  @alias('loadTask.lastSuccessful.value') visualization;
  @bool('visualization') loaded;

  async load() {
    await this.loadTask.perform();
  }

  @(task(function* () {
    let api = yield loadJsApi();
    yield loadCoreChart(api);
    return api.visualization;
  }).keepLatest())
  loadTask;
}

async function loadScript(src) {
  await new Promise((resolve, reject) => {
    const script = document.createElement('script');
    script.src = src;
    script.onload = resolve;
    script.onerror = reject;
    document.body.appendChild(script);
  });
}

async function loadJsApi() {
  if (!window.google) {
    await loadScript('https://www.google.com/jsapi');
  }
  return window.google;
}

async function loadCoreChart(api) {
  await new Promise(resolve => {
    api.load('visualization', '1.0', {
      packages: ['corechart'],
      callback: resolve,
    });
  });
}
