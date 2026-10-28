'use strict';
const { ServiceError } = require('./models');

function identifier(value, field = 'id') {
  if (typeof value !== 'string' || !/^[a-z][a-z0-9-]{1,63}$/.test(value)) {
    throw new ServiceError(`${field} must be a lowercase identifier`);
  }
  return value;
}

function title(value) {
  if (typeof value !== 'string') throw new ServiceError('Title must be text');
  const clean = value.trim().replace(/\s+/g, ' ');
  if (clean.length < 3 || clean.length > 160) throw new ServiceError('Title length out of range');
  return clean;
}

function money(value) {
  if (!Number.isSafeInteger(value) || value < 0 || value > 100000000) {
    throw new ServiceError('Amount must be integer cents within range');
  }
  return value;
}

function payload(input, kinds) {
  const fields = ['id', 'kind', 'owner', 'title', 'amount', 'tags'];
  if (Object.keys(input).some(k => !fields.includes(k))) throw new ServiceError('Unknown record field');
  if (!kinds.includes(input.kind)) throw new ServiceError('Unsupported record kind');
  const tags = input.tags || [];
  if (!Array.isArray(tags) || tags.length > 10) throw new ServiceError('Invalid tags');
  return { id: identifier(input.id), kind: input.kind, owner: identifier(input.owner, 'owner'),
    title: title(input.title), amount: money(input.amount ?? 0),
    tags: [...new Set(tags.map(t => identifier(t, 'tag')))].sort() };
}

module.exports = { identifier, title, money, payload };
