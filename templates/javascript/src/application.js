'use strict';
const { Record, ServiceError, event } = require('./models');
const validation = require('./validation');
const policy = require('./policy_registry');

class Application {
  constructor(repository, settings) {
    this.repository = repository;
    this.settings = settings;
    this.events = [];
    this.outbox = [];
    this.metrics = [];
  }

  principal(session) {
    const user = policy.auth.authenticate(session);
    if (!user) throw new ServiceError('Verified session required', 403);
    return policy.roles.effectivePrincipal(user);
  }

  recordEvent(action, user, record) {
    return policy.audit.appendEvent(this.events, event(action, user.id, record.id));
  }

  create(session, input, requestKey) {
    const user = this.principal(session);
    if (!user.can('operator')) throw new ServiceError('operator role required', 403);
    const data = validation.payload(input, this.settings.record_kinds);
    if (data.owner !== user.id) throw new ServiceError('Owner mismatch', 403);
    const record = this.repository.insert(new Record(data), requestKey);
    this.recordEvent('record.created', user, record);
    return record;
  }

  list(session, options = {}) {
    const user = this.principal(session);
    return this.repository.list({ ...options, owner: user.can('reviewer') ? undefined : user.id });
  }

  transition(session, id, target, version) {
    const user = this.principal(session);
    if (!user.can('reviewer')) throw new ServiceError('reviewer role required', 403);
    const record = this.repository.get(id);
    const updated = this.repository.save(record.transition(target), version);
    this.recordEvent(`record.${target}`, user, updated);
    return updated;
  }

  invoice(session, id) {
    const user = this.principal(session);
    const record = this.repository.get(id);
    if (record.owner !== user.id && !user.can('reviewer')) throw new ServiceError('Owner mismatch', 403);
    return policy.billing.buildInvoice(record);
  }

  export(session) {
    const user = this.principal(session);
    const rows = this.repository.snapshot().filter(r => r.owner === user.id);
    const plan = policy.exports.planExport(rows, user.id, this.settings.max_export_rows);
    this.outbox.push(...plan.deliveries);
    return plan;
  }

  health() {
    return { status: 'ready', records: this.repository.snapshot().length,
      pendingDeliveries: this.outbox.length, auditEvents: this.events.length };
  }
}

module.exports = { Application };
