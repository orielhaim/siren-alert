export function settingsWriter<T>(write: (value: T) => Promise<void>) {
  let queue = Promise.resolve();
  return {
    write(value: T) {
      const pending = queue.then(() => write(value));
      queue = pending.catch(() => {});
      return pending;
    },
    settled() {
      return queue;
    },
  };
}
