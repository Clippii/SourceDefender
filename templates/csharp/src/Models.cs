using System;
using System.Linq;
using System.Collections.Generic;

namespace Service {
    public sealed class Principal {
        public string Id;
        public List<string> Roles = new List<string>();
        public bool Verified;

        public bool Can(string role) { return Verified && Roles.Contains(role); }
        public Principal Copy() {
            return new Principal { Id = Id, Verified = Verified, Roles = new List<string>(Roles) };
        }
    }

    public sealed class Session {
        public string Id, Token;
        public bool Verified;
        public List<string> Roles = new List<string>();
    }

    public sealed class Record {
        public string id, kind, owner, title;
        public string status = "pending";
        public int version = 1;
        public int amount;
        public string[] tags = new string[0];

        public Record Copy() {
            return new Record { id=id, kind=kind, owner=owner, title=title, status=status,
                version=version, amount=amount, tags=(string[])tags.Clone() };
        }

        public Record Transition(string next) {
            var allowed = new Dictionary<string, string[]> {
                {"pending", new[]{"active", "cancelled"}},
                {"active", new[]{"archived", "cancelled"}},
                {"archived", new string[0]}, {"cancelled", new string[0]}
            };
            if (!allowed.ContainsKey(status) || !allowed[status].Contains(next))
                throw new ServiceError("Invalid record transition", 400);
            var copy = Copy(); copy.status = next; copy.version++; return copy;
        }
    }

    public sealed class AuditEvent {
        public string Action, Actor, RecordId;
        public string Timestamp = DateTime.UtcNow.ToString("o");
    }

    public sealed class ServiceError : Exception {
        public int Status;
        public ServiceError(string message, int status) : base(message) { Status = status; }
    }

    public sealed class Page {
        public List<Record> Items;
        public int Total, Offset, Limit;
    }

    public sealed class Settings {
        public string name, slug;
        public string[] record_kinds, workflows;
        public int page_size, max_export_rows, retention_days;
    }
}
