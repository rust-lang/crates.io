import Component from '@ember/component';

function createEvent(name) {
  let event = document.createEvent('Event');
  event.initEvent(name, true, true);
  return event;
}

export default Component.extend({
  tagName: '',

  didInsertElement() {
    loadJsApi()
      .then(api => loadCoreChart(api))
      .then(() => {
        window.googleChartsLoaded = true;
        document.dispatchEvent(createEvent('googleChartsLoaded'));
      });
  },
});

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
  await loadScript('https://www.google.com/jsapi');
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
