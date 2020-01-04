import Service, { inject as service } from '@ember/service';

import ajax from '../utils/ajax';

const KEY = 'ajax-cache';

export default class FetcherService extends Service {
  @service fastboot;

  get(url) {
    let shoebox = this.fastboot.shoebox;
    if (!shoebox) {
      return;
    }
    let cache = shoebox.retrieve(KEY) || {};
    return cache[url];
  }

  put(url, obj) {
    let fastboot = this.fastboot;
    let shoebox = this.fastboot.shoebox;
    if (!(shoebox && fastboot.isFastBoot)) {
      return;
    }

    let cache = shoebox.retrieve(KEY) || {};
    cache[url] = deepCopy(obj);
    shoebox.put(KEY, cache);
  }

  ajax(url) {
    let resp = this.get(url);
    if (resp) {
      return resp;
    }

    return ajax(url).then(resp => {
      this.put(url, resp);
      return resp;
    });
  }
}

function deepCopy(obj) {
  return JSON.parse(JSON.stringify(obj));
}
