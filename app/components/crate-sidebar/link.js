import Component from '@glimmer/component';

export default class CrateSidebarLink extends Component {
  get simplifiedUrl() {
    let { url } = this.args;
    return simplifyUrl(url);
  }

  get text() {
    // Add zero-width space characters around `/` and `.` characters
    // to allow more line breaks in the middle of long URLs
    return this.simplifiedUrl.replace(/([./])/g, '\u200B$1\u200B');
  }

  get isDocsRs() {
    return this.simplifiedUrl.startsWith('docs.rs/');
  }

  get isGitHub() {
    return this.simplifiedUrl.startsWith('github.com/');
  }
}

export function simplifyUrl(url) {
  if (url.startsWith('https://')) {
    url = url.slice('https://'.length);
  }
  if (url.startsWith('www.')) {
    url = url.slice('www.'.length);
  }
  if (url.endsWith('/')) {
    url = url.slice(0, -1);
  }
  if (url.startsWith('github.com/') && url.endsWith('.git')) {
    url = url.slice(0, -4);
  }

  return url;
}
