import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { NotificationsState } from './notifications.svelte';

describe('NotificationsState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts with empty content', () => {
    let state = new NotificationsState();
    expect(state.content).toEqual([]);
  });

  it('adds info notification', () => {
    let state = new NotificationsState();
    state.info('Test message');

    expect(state.content).toHaveLength(1);
    expect(state.content[0]?.type).toBe('info');
    expect(state.content[0]?.message).toBe('Test message');
    expect(state.content[0]?.autoClear).toBe(false);
    expect(state.content[0]?.clearDuration).toBe(10_000);
  });

  it('adds success notification', () => {
    let state = new NotificationsState();
    state.success('Success!');

    expect(state.content).toHaveLength(1);
    expect(state.content[0]?.type).toBe('success');
    expect(state.content[0]?.message).toBe('Success!');
  });

  it('adds warning notification', () => {
    let state = new NotificationsState();
    state.warning('Be careful');

    expect(state.content).toHaveLength(1);
    expect(state.content[0]?.type).toBe('warning');
    expect(state.content[0]?.message).toBe('Be careful');
  });

  it('adds error notification', () => {
    let state = new NotificationsState();
    state.error('Something went wrong');

    expect(state.content).toHaveLength(1);
    expect(state.content[0]?.type).toBe('error');
    expect(state.content[0]?.message).toBe('Something went wrong');
  });

  it('supports autoClear: false option', () => {
    let state = new NotificationsState();
    state.info('Persistent message', { autoClear: false });

    expect(state.content[0]?.autoClear).toBe(false);
  });

  it('supports custom clearDuration option', () => {
    let state = new NotificationsState();
    state.info('Quick message', { clearDuration: 3000 });

    expect(state.content[0]?.clearDuration).toBe(3000);
  });

  it('returns notification from helper methods', () => {
    let state = new NotificationsState();
    let notification = state.info('Test');

    expect(notification).toBe(state.content[0]);
    expect(notification.message).toBe('Test');
  });

  it('removeNotification marks for dismissal then removes', () => {
    let state = new NotificationsState();
    state.info('First');
    state.info('Second');
    state.info('Third');

    let second = state.content[1]!;
    state.removeNotification(second);

    // Notification is marked for dismissal but not yet removed
    expect(state.content).toHaveLength(3);
    expect(state.content[1]?.dismiss).toBe(true);

    // After 500ms, the notification is actually removed
    vi.advanceTimersByTime(500);

    expect(state.content).toHaveLength(2);
    expect(state.content.map(n => n.message)).toEqual(['First', 'Third']);
  });

  it('clearAll removes all notifications', () => {
    let state = new NotificationsState();
    state.info('One');
    state.success('Two');
    state.error('Three');

    expect(state.content).toHaveLength(3);

    let result = state.clearAll();

    // Returns this for chaining
    expect(result).toBe(state);

    // All marked for dismissal
    expect(state.content.every(n => n.dismiss)).toBe(true);

    // After 500ms, all removed
    vi.advanceTimersByTime(500);
    expect(state.content).toEqual([]);
  });

  it('maintains notification order (newest last)', () => {
    let state = new NotificationsState();
    state.info('First');
    state.success('Second');
    state.error('Third');

    expect(state.content.map(n => n.message)).toEqual(['First', 'Second', 'Third']);
  });

  it('handles removing undefined notification gracefully', () => {
    let state = new NotificationsState();
    state.info('Existing');

    // @ts-expect-error testing invalid input
    state.removeNotification(undefined);

    expect(state.content).toHaveLength(1);
  });

  it('addNotification throws if no message provided', () => {
    let state = new NotificationsState();

    // @ts-expect-error testing invalid input
    expect(() => state.addNotification({})).toThrow('No notification message set');
  });

  it('setDefaultAutoClear changes default', () => {
    let state = new NotificationsState();
    state.setDefaultAutoClear(true);
    state.info('Test');

    expect(state.content[0]?.autoClear).toBe(true);
  });

  it('setDefaultClearDuration changes default', () => {
    let state = new NotificationsState();
    state.setDefaultClearDuration(5000);
    state.info('Test');

    expect(state.content[0]?.clearDuration).toBe(5000);
  });
});
