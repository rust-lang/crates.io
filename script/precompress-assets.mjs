import { createReadStream, createWriteStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { pipeline } from 'node:stream/promises';
import { constants, createBrotliCompress, createGzip } from 'node:zlib';

import { globby } from 'globby';

let paths = await globby(
  ['**/*.css', '**/*.html', '**/*.js', '**/*.map', '**/*.svg', '**/*.txt', '**/*.wasm', '**/*.xml'],
  { cwd: 'dist' },
);

for (let path of paths) {
  let fullPath = `dist/${path}`;
  console.log(`Compressing ${fullPath}…`);

  let gzipPath = fullPath + '.gz';
  await pipeline(createReadStream(fullPath), createGzip({ level: 9 }), createWriteStream(gzipPath));

  let brPath = fullPath + '.br';
  await pipeline(
    createReadStream(fullPath),
    createBrotliCompress({
      params: {
        [constants.BROTLI_PARAM_QUALITY]: constants.BROTLI_MAX_QUALITY,
        [constants.BROTLI_PARAM_SIZE_HINT]: (await stat(fullPath)).size,
      },
    }),
    createWriteStream(brPath),
  );
}

console.log('\nCompression completed');
