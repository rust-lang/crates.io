import Service from '@ember/service';

import { getItem, setItem } from '../utils/local-storage';

const BANNER_MESSAGE_STORAGE_KEY = 'dismissed-banner-messages';

/**
 * Tracking for previously dismissed site banners using local storage.
 */
export default class BannerMessageService extends Service {
  /**
   * Mark the given message content as being dismissed.
   *
   * @param {string} text
   * @returns {void}
   */
  async dismiss(text) {
    let id = await this.#hash(text);
    let set = this.#readDismissedSet();

    set.add(id);
    this.#writeDismissedSet(set);
  }

  /**
   * Check if the given message content has previously been dismissed.
   *
   * @param {string} text
   */
  async previouslyDismissed(text) {
    let id = await this.#hash(text);
    let set = this.#readDismissedSet();

    return set.has(id);
  }

  /**
   * @param {string} text
   * @returns {Promise<string>}
   */
  async #hash(text) {
    let input = new TextEncoder().encode(text);
    let output = await window.crypto.subtle.digest('SHA-256', input);
    return new Uint8Array(output).toHex();
  }

  /**
   * @returns {Set<string>}
   */
  #readDismissedSet() {
    let raw = getItem(BANNER_MESSAGE_STORAGE_KEY);
    return raw ? new Set(raw.split(',')) : new Set();
  }

  /**
   * @param {Set<string>} set
   * @returns {void}
   */
  #writeDismissedSet(set) {
    setItem(BANNER_MESSAGE_STORAGE_KEY, [...set.values()].join(','));
  }
}
