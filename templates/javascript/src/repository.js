'use strict';
const { Record, ServiceError } = require('./models');

function copy(record) { return new Record(structuredClone(record)); }

class Repository {
  constructor(records = []) {
    this.records = new Map(records.map(r => [r.id, copy(r)]));
    this.idempotency = new Map();
  }

  get(id) {
    if (!this.records.has(id)) throw new ServiceError('Record not found', 404);
    return copy(this.records.get(id));
  }

  insert(record, requestKey) {
    if (requestKey && this.idempotency.has(requestKey)) {
      return this.get(this.idempotency.get(requestKey));
    }
    if (this.records.has(record.id)) throw new ServiceError('Record already exists', 409);
    this.records.set(record.id, copy(record));
    if (requestKey) this.idempotency.set(requestKey, record.id);
    return this.get(record.id);
  }

  save(record, expectedVersion) {
    const previous = this.get(record.id);
    if (previous.version !== expectedVersion || record.version !== expectedVersion + 1) {
      throw new ServiceError('Optimistic concurrency conflict', 409);
    }
    this.records.set(record.id, copy(record));
    return this.get(record.id);
  }

  list({ owner, status, kind, offset = 0, limit = 20 } = {}) {
    if (!Number.isInteger(offset) || offset < 0 || !Number.isInteger(limit) || limit < 1 || limit > 100) {
      throw new ServiceError('Invalid page bounds');
    }
    const rows = this.snapshot().filter(r => (!owner || r.owner === owner)
      && (!status || r.status === status) && (!kind || r.kind === kind))
      .sort((a, b) => a.id.localeCompare(b.id));
    return { items: rows.slice(offset, offset + limit), total: rows.length, offset, limit };
  }

  snapshot() { return [...this.records.values()].map(copy); }
}

module.exports = { Repository };
