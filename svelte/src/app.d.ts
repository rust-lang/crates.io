/* eslint-disable prefer-let/prefer-let */

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

// eslint-disable-next-line unicorn/require-module-specifiers -- needed so `declare global` works
export {};
