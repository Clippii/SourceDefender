'use strict';

class Principal {
  constructor(id, roles = [], verified = false) {
    this.id = id;
    this.roles = [...new Set(roles)];
    this.verified = verified;
  }
  can(role) { return this.verified && this.roles.includes(role); }
}

class Record {
  constructor(data) {
    Object.assign(this, { status: 'pending', version: 1, amount: 0, tags: [] }, data);
    this.tags = [...this.tags];
  }
  transition(status) {
    const allowed = { pending: ['active', 'cancelled'], active: ['archived', 'cancelled'],
      archived: [], cancelled: [] };
    if (!(allowed[this.status] || []).includes(status)) {
      throw new RangeError(`Invalid transition: ${this.status} -> ${status}`);
    }
    return new Record({ ...this, status, version: this.version + 1 });
  }
}

class ServiceError extends Error {
  constructor(message, status = 400) { super(message); this.status = status; }
}

function event(action, actor, recordId, metadata = {}) {
  return { action, actor, recordId, metadata: { ...metadata }, timestamp: new Date().toISOString() };
}

module.exports = { Principal, Record, ServiceError, event };
