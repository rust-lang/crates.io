import Service, { service } from '@ember/service';

// eslint-disable-next-line no-unused-vars
import CookiesService from 'ember-cookies/services/cookies';

const BANNER_MESSAGE_COOKIE_NAME = 'dismissed-banner-messages';

/**
 * Tracking for previously dismissed site banners using cookies.
 */
export default class BannerMessageService extends Service {
  /** @type {CookiesService} */
  @service cookies;

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
    let raw = this.cookies.read(BANNER_MESSAGE_COOKIE_NAME);
    return raw === undefined ? new Set() : new Set(raw.split(','));
  }

  /**
   * @param {Set<string>} set
   * @returns {void}
   */
  #writeDismissedSet(set) {
    this.cookies.write(BANNER_MESSAGE_COOKIE_NAME, [...set.values()].join(','));
  }
}
