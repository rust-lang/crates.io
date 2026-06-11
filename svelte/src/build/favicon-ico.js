import sharp from 'sharp';

import encodeIco from './encode-ico.js';

const FILE_NAME = 'favicon.ico';

// Sizes a browser picks from for tabs, bookmarks, and the taskbar.
const SIZES = [16, 32, 48];

/**
 * Renders a square source image into ICO bytes with PNG-compressed frames at
 * the standard favicon sizes.
 *
 * @param {string} source path to the square source image
 * @returns {Promise<Buffer>}
 */
async function render(source) {
  let frames = await Promise.all(
    SIZES.map(async size => {
      let data = await sharp(source)
        .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
        .png({ quality: 80, compressionLevel: 9, palette: true })
        .toBuffer();
      return { size, data };
    }),
  );
  return encodeIco(frames);
}

/**
 * Generates `/favicon.ico` from a source image, emitting it as a build asset
 * and serving it from the dev server so the same bytes are used everywhere.
 *
 * @param {{ source: string }} options `source` is the path to the square
 *   source image the icon is derived from
 * @returns {import('vite').Plugin}
 */
export default function faviconIco({ source }) {
  /** @type {Promise<Buffer> | undefined} */
  let rendered;
  let build = () => (rendered ??= render(source));

  return {
    name: 'favicon-ico',

    async generateBundle() {
      // `this` is the Rollup plugin context provided by the hook API.
      /* eslint-disable unicorn/no-this-outside-of-class */
      // SvelteKit runs a separate SSR build that should not emit the asset.
      if (this.environment && this.environment.name !== 'client') return;
      this.emitFile({ type: 'asset', fileName: FILE_NAME, source: await build() });
      /* eslint-enable unicorn/no-this-outside-of-class */
    },

    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (req.url?.split('?', 1)[0] !== `/${FILE_NAME}`) return next();
        res.setHeader('Content-Type', 'image/x-icon');
        res.end(await build());
      });
    },
  };
}
