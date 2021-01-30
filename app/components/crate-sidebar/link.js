import Component from '@glimmer/component';

export default class CrateSidebarLink extends Component {
  get text() {
    let { url } = this.args;
    if (url.startsWith('https://')) {
      url = url.slice('https://'.length);
    }
    if (url.startsWith('www.')) {
      url = url.slice('www.'.length);
    }

    return url;
  }
}
