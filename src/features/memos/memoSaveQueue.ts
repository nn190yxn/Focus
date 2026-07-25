export class LatestMemoSaveQueue<T> {
  private active = false;
  private queued: T | null = null;

  enqueue(value: T): T | null {
    if (this.active) {
      this.queued = value;
      return null;
    }
    this.active = true;
    return value;
  }

  complete(): T | null {
    if (this.queued !== null) {
      const next = this.queued;
      this.queued = null;
      return next;
    }
    this.active = false;
    return null;
  }

  isActive(): boolean {
    return this.active;
  }
}
