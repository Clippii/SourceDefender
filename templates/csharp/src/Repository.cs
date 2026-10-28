using System;
using System.Linq;
using System.Collections.Generic;

namespace Service {
    public sealed class Repository {
        readonly object gate = new object();
        readonly Dictionary<string, Record> records = new Dictionary<string, Record>();
        readonly Dictionary<string, string> idempotency = new Dictionary<string, string>();

        public Repository(IEnumerable<Record> initial) {
            foreach (var record in initial) records.Add(record.id, record.Copy());
        }

        public Record Get(string id) {
            lock(gate) {
                if (!records.ContainsKey(id)) throw new ServiceError("Record not found", 404);
                return records[id].Copy();
            }
        }

        public Record Insert(Record record, string requestKey) {
            lock(gate) {
                if (!string.IsNullOrEmpty(requestKey) && idempotency.ContainsKey(requestKey))
                    return Get(idempotency[requestKey]);
                if (records.ContainsKey(record.id)) throw new ServiceError("Record already exists", 409);
                records.Add(record.id, record.Copy());
                if (!string.IsNullOrEmpty(requestKey)) idempotency.Add(requestKey, record.id);
                return Get(record.id);
            }
        }

        public Record Save(Record record, int expectedVersion) {
            lock(gate) {
                var previous = Get(record.id);
                if (previous.version != expectedVersion || record.version != expectedVersion + 1)
                    throw new ServiceError("Optimistic concurrency conflict", 409);
                records[record.id] = record.Copy(); return Get(record.id);
            }
        }

        public Page List(string owner, string status, string kind, int offset, int limit) {
            if (offset < 0 || limit < 1 || limit > 100)
                throw new ServiceError("Invalid page bounds", 400);
            lock(gate) {
                var rows = records.Values.Where(r => (owner == null || r.owner == owner)
                    && (status == null || r.status == status) && (kind == null || r.kind == kind))
                    .OrderBy(r => r.id).ToList();
                return new Page { Items=rows.Skip(offset).Take(limit).Select(r=>r.Copy()).ToList(),
                    Total=rows.Count, Offset=offset, Limit=limit };
            }
        }

        public List<Record> Snapshot() {
            lock(gate) { return records.Values.Select(r=>r.Copy()).ToList(); }
        }
    }
}
