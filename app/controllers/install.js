import Controller from '@ember/controller';

function link(target) {
  return `https://static.rust-lang.org/cargo-dist/cargo-nightly-${target}.tar.gz`;
}

export default Controller.extend({
  linux64: link('x86_64-unknown-linux-gnu'),
  linux32: link('i686-unknown-linux-gnu'),
  mac64: link('x86_64-apple-darwin'),
  mac32: link('i686-apple-darwin'),
  win64: link('x86_64-pc-windows-gnu'),
  win32: link('i686-pc-windows-gnu'),
});
