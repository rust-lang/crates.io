import { createContext } from 'svelte';

const ANIMATION_INTERVAL_MS = 200;
const FADE_OUT_DURATION_MS = ANIMATION_INTERVAL_MS * 2;

export class ProgressState {
  style = $state('');

  #intervalId: ReturnType<typeof setInterval> | null = null;
  #timeoutId: ReturnType<typeof setTimeout> | null = null;
  #progress = 0;
  #activeCount = 0;

  trackPromise(promise: Promise<unknown>): void {
    this.#activeCount++;

    if (this.#activeCount === 1) {
      this.#startAnimation();
    }

    promise.then(
      () => this.#onPromiseSettled(),
      () => this.#onPromiseSettled(),
    );
  }

  #startAnimation(): void {
    this.#cleanup();
    this.#progress = 0;
    this.style = 'width: 0%';

    this.#intervalId = setInterval(() => this.#advanceProgress(), ANIMATION_INTERVAL_MS);
  }

  #getProgressIncrement(): number {
    if (this.#progress < 0.2) {
      return 0.1;
    } else if (this.#progress < 0.5) {
      return 0.04;
    } else if (this.#progress < 0.8) {
      return 0.02;
    } else if (this.#progress < 0.99) {
      return 0.005;
    } else {
      return 0;
    }
  }

  #advanceProgress(): void {
    this.#progress = Math.min(this.#progress + this.#getProgressIncrement(), 0.998);
    this.style = `transition: width ${ANIMATION_INTERVAL_MS}ms linear; width: ${this.#progress * 100}%`;
  }

  #onPromiseSettled(): void {
    this.#activeCount--;

    if (this.#activeCount === 0) {
      this.#complete();
    }
  }

  #complete(): void {
    if (this.#intervalId !== null) {
      clearInterval(this.#intervalId);
      this.#intervalId = null;
    }

    this.style = `transition: width ${ANIMATION_INTERVAL_MS}ms linear; width: 100%`;
    this.#timeoutId = setTimeout(() => this.#fadeOut(), ANIMATION_INTERVAL_MS);
  }

  #fadeOut(): void {
    this.style = `transition: opacity ${FADE_OUT_DURATION_MS}ms linear; width: 100%; opacity: 0`;
    this.#timeoutId = setTimeout(() => this.#reset(), FADE_OUT_DURATION_MS);
  }

  #reset(): void {
    this.style = '';
    this.#timeoutId = null;
  }

  #cleanup(): void {
    if (this.#intervalId !== null) {
      clearInterval(this.#intervalId);
      this.#intervalId = null;
    }

    if (this.#timeoutId !== null) {
      clearTimeout(this.#timeoutId);
      this.#timeoutId = null;
    }
  }
}

export interface ProgressContext {
  readonly style: string;
  trackPromise: (promise: Promise<unknown>) => void;
}

export const [getProgressContext, setProgressContext] = createContext<ProgressContext>();
