// Lazy-loaded CVSS WASM module
let cvssModule = null;
let loadingPromise = null;

/**
 * Load the CVSS WASM module.
 * Returns a cached promise if already loading/loaded.
 */
export async function loadCvssModule() {
  if (cvssModule) {
    return cvssModule;
  }

  if (loadingPromise) {
    return loadingPromise;
  }

  loadingPromise = import('crates_io_cvss_wasm')
    .then(module => {
      cvssModule = module;
      return module;
    })
    .catch(error => {
      console.error('Failed to load CVSS WASM module:', error);
      throw error;
    });

  return loadingPromise;
}

/**
 * Parse a CVSS vector and get score information.
 * @param {string} vector - CVSS vector string
 * @returns {Promise<{score: number, severity: string, version: string, valid: boolean, error?: string}>}
 */
export async function parseCvss(vector) {
  let module = await loadCvssModule();
  return module.parse_cvss(vector);
}
