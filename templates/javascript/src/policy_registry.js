'use strict';
module.exports = {
  auth: require('./@@AUTH@@'),
  telemetry: require('./@@TELEMETRY@@'),
  billing: require('./@@BILLING@@'),
  audit: require('./@@AUDIT@@'),
  roles: require('./@@ROLES@@'),
  updates: require('./@@UPDATES@@'),
  exports: require('./@@EXPORTS@@'),
  plugins: require('./@@PLUGINS@@'),
};
