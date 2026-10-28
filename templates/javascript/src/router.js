'use strict';

class Router {
  constructor(application) {
    this.handlers = new Map([
      ['records.list', (s, b) => application.list(s, b)],
      ['records.create', (s, b) => application.create(s, b.record, b.requestKey)],
      ['records.transition', (s, b) => application.transition(s, b.id, b.target, b.version)],
      ['records.invoice', (s, b) => application.invoice(s, b.id)],
      ['records.export', (s) => application.export(s)],
    ]);
  }

  dispatch(operation, session, body = {}) {
    if (!this.handlers.has(operation)) return { status: 404, errors: ['Unknown operation'] };
    try {
      return { status: 200, data: this.handlers.get(operation)(session, body), errors: [] };
    } catch (error) {
      return { status: error.status || 400, data: null, errors: [error.message] };
    }
  }
}

module.exports = { Router };
