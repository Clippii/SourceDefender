'use strict';

class JobQueue {
  constructor(maxAttempts = 3) {
    this.maxAttempts = maxAttempts;
    this.pending = [];
    this.known = new Set();
    this.completed = new Map();
    this.deadLetters = [];
  }

  enqueue(id, kind, payload) {
    if (this.known.has(id)) return false;
    this.known.add(id);
    this.pending.push({ id, kind, payload: structuredClone(payload), attempts: 0,
      retryAt: 0, state: 'pending', error: '' });
    return true;
  }

  async runOnce(handlers, now = Date.now()) {
    const job = this.pending.shift();
    if (!job) return null;
    if (job.retryAt > now) { this.pending.push(job); return null; }
    job.attempts += 1;
    try {
      if (!handlers[job.kind]) throw new Error('Unknown job kind');
      await handlers[job.kind](job.payload);
      job.state = 'completed';
      this.completed.set(job.id, job);
    } catch (error) {
      job.error = error.message;
      if (job.attempts >= this.maxAttempts) {
        job.state = 'failed';
        this.deadLetters.push(job);
      } else {
        job.state = 'retry';
        job.retryAt = now + 1000 * (2 ** job.attempts);
        this.pending.push(job);
      }
    }
    return job;
  }

  statistics() {
    return { pending: this.pending.length, completed: this.completed.size,
      deadLetters: this.deadLetters.length };
  }
}

module.exports = { JobQueue };
