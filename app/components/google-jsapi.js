import Component from '@ember/component';

function createEvent(name) {
  let event = document.createEvent('Event');
  event.initEvent(name, true, true);
  return event;
}

export default Component.extend({
  tagName: '',

  didInsertElement() {
    let script = document.createElement('script');
    script.onload = () => {
      window.google.load('visualization', '1.0', {
        packages: ['corechart'],
        callback() {
          window.googleChartsLoaded = true;
          document.dispatchEvent(createEvent('googleChartsLoaded'));
        },
      });
    };
    document.body.appendChild(script);
    script.src = 'https://www.google.com/jsapi';
  },
});
