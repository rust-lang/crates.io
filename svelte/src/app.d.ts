/* eslint-disable prefer-let/prefer-let */

import '@poppanator/sveltekit-svg/dist/svg.d.ts';

// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
  namespace App {
    interface Error {
      message: string;
      details?: string;
      tryAgain?: boolean;
      loginNeeded?: boolean;
    }

    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }

  const __TEST__: boolean;
}

export {};
