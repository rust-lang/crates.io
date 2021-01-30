import Component from '@glimmer/component';

export default class CrateSidebarLink extends Component {
  get text() {
    let { url } = this.args;
    if (url.startsWith('https://')) {
      url = url.slice('https://'.length);
    }

    return url;
  }
}
