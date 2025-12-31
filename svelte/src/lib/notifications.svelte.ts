import { createContext } from 'svelte';

export type NotificationType = 'info' | 'success' | 'warning' | 'error';

export interface NotificationOptions {
  autoClear?: boolean;
  clearDuration?: number;
}

export class Notification {
  type: NotificationType;
  message: string;
  autoClear: boolean;
  clearDuration: number;
  remaining: number;
  startTime: number | undefined;
  timer: ReturnType<typeof setTimeout> | undefined;
  dismiss = $state(false);

  constructor(options: { type: NotificationType; message: string; autoClear: boolean; clearDuration: number }) {
    this.type = options.type;
    this.message = options.message;
    this.autoClear = options.autoClear;
    this.clearDuration = options.clearDuration;
    this.remaining = options.clearDuration;
  }
}

const DEFAULT_AUTO_CLEAR = !__TEST__;
const DEFAULT_CLEAR_DURATION = 10_000;

export class NotificationsState {
  content = $state.raw<Notification[]>([]);

  #defaultAutoClear = DEFAULT_AUTO_CLEAR;
  #defaultClearDuration = DEFAULT_CLEAR_DURATION;

  addNotification(options: { message: string; type?: NotificationType } & NotificationOptions): Notification {
    if (!options.message) {
      throw new Error('No notification message set');
    }

    let notification = new Notification({
      type: options.type ?? 'info',
      message: options.message,
      autoClear: options.autoClear ?? this.#defaultAutoClear,
      clearDuration: options.clearDuration ?? this.#defaultClearDuration,
    });

    this.content = [...this.content, notification];

    if (notification.autoClear) {
      this.setupAutoClear(notification);
    }

    return notification;
  }

  info(message: string, options?: NotificationOptions): Notification {
    return this.addNotification({ ...options, message, type: 'info' });
  }

  success(message: string, options?: NotificationOptions): Notification {
    return this.addNotification({ ...options, message, type: 'success' });
  }

  warning(message: string, options?: NotificationOptions): Notification {
    return this.addNotification({ ...options, message, type: 'warning' });
  }

  error(message: string, options?: NotificationOptions): Notification {
    return this.addNotification({ ...options, message, type: 'error' });
  }

  removeNotification(notification: Notification): void {
    if (!notification) return;

    notification.dismiss = true;

    setTimeout(() => {
      this.content = this.content.filter(n => n !== notification);
    }, 500);
  }

  clearAll(): this {
    for (let notification of this.content) {
      this.removeNotification(notification);
    }
    return this;
  }

  setupAutoClear(notification: Notification): void {
    if (!notification.autoClear) return;

    notification.startTime = Date.now();
    notification.timer = setTimeout(() => {
      if (this.content.includes(notification)) {
        this.removeNotification(notification);
      }
    }, notification.remaining);
  }

  pauseAutoClear(notification: Notification): void {
    if (!notification.autoClear || !notification.timer) return;

    clearTimeout(notification.timer);
    notification.timer = undefined;

    let elapsed = Date.now() - (notification.startTime ?? Date.now());
    notification.remaining = Math.max(0, notification.remaining - elapsed);
    notification.startTime = undefined;
  }

  setDefaultAutoClear(autoClear: boolean): void {
    this.#defaultAutoClear = autoClear;
  }

  setDefaultClearDuration(clearDuration: number): void {
    this.#defaultClearDuration = clearDuration;
  }
}

export interface NotificationsContext {
  readonly content: Notification[];
  addNotification: (options: { message: string; type?: NotificationType } & NotificationOptions) => Notification;
  info: (message: string, options?: NotificationOptions) => Notification;
  success: (message: string, options?: NotificationOptions) => Notification;
  warning: (message: string, options?: NotificationOptions) => Notification;
  error: (message: string, options?: NotificationOptions) => Notification;
  removeNotification: (notification: Notification) => void;
  clearAll: () => NotificationsState;
  setupAutoClear: (notification: Notification) => void;
  pauseAutoClear: (notification: Notification) => void;
  setDefaultAutoClear: (autoClear: boolean) => void;
  setDefaultClearDuration: (clearDuration: number) => void;
}

export const [getNotifications, setNotifications] = createContext<NotificationsContext>();
