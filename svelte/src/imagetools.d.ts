/* eslint-disable prefer-let/prefer-let */

// Types `vite-imagetools` imports tagged with a trailing `&imagetools` marker.
// The marker is ignored by imagetools and exists only so this declaration can
// match. The real output (a hashed URL or an inlined `data:` URI, both strings)
// is selected by the other query directives.
//
// See https://github.com/JonasKruckenberg/imagetools/issues/160#issuecomment-1009292026
//
// Keep this in its own file with no top-level `import` or `export`. TypeScript
// only applies a `declare module` globally when the file has neither, so adding
// one (or moving this into `app.d.ts`) silently stops it from matching.
declare module '*&imagetools' {
  const src: string;
  export default src;
}
