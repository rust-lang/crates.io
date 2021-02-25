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
    if (url.endsWith('/')) {
      url = url.slice(0, -1);
    }

    return url;
  }

  get isDocsRs() {
    return this.text.startsWith('docs.rs/');
  }

  get isGitHub() {
    return this.text.startsWith('github.com/');
  }
}
