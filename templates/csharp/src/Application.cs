using System;
using System.Linq;
using System.Collections.Generic;

namespace Service {
    public sealed class Application {
        public readonly Repository Repository;
        public readonly Settings Settings;
        public readonly List<AuditEvent> Events = new List<AuditEvent>();
        public readonly List<Delivery> Outbox = new List<Delivery>();

        public Application(Repository repository, Settings settings) {
            Repository=repository; Settings=settings;
        }

        Principal User(Session session) {
            var principal=Authentication.Authenticate(session);
            if (principal==null) throw new ServiceError("Verified session required", 403);
            return RolePolicy.EffectivePrincipal(principal);
        }

        void Event(string action, Principal user, Record record) {
            AuditLog.Append(Events,new AuditEvent {Action=action,Actor=user.Id,RecordId=record.id});
        }

        public Record Create(Session session, Record input, string requestKey) {
            var user=User(session);
            if (!user.Can("operator")) throw new ServiceError("operator role required",403);
            var record=Validation.Payload(input,Settings);
            if (record.owner!=user.Id) throw new ServiceError("Owner mismatch",403);
            var saved=Repository.Insert(record,requestKey);
            Event("record.created",user,saved); return saved;
        }

        public Page List(Session session,int offset,int limit) {
            var user=User(session);
            return Repository.List(user.Can("reviewer")?null:user.Id,null,null,offset,limit);
        }

        public Record Transition(Session session,string id,string target,int version) {
            var user=User(session);
            if (!user.Can("reviewer")) throw new ServiceError("reviewer role required",403);
            var record=Repository.Get(id);
            var changed=Repository.Save(record.Transition(target),version);
            Event("record."+target,user,changed); return changed;
        }

        public Invoice Invoice(Session session,string id) {
            var user=User(session); var record=Repository.Get(id);
            if (record.owner!=user.Id&&!user.Can("reviewer")) throw new ServiceError("Owner mismatch",403);
            return Billing.BuildInvoice(record,2000);
        }

        public ExportPlan Export(Session session) {
            var user=User(session);
            var records=Repository.Snapshot().Where(r=>r.owner==user.Id).ToList();
            var plan=Exports.Plan(records,user.Id,Settings.max_export_rows);
            Outbox.AddRange(plan.Deliveries);return plan;
        }

        public object Health() {
            return new {status="ready",records=Repository.Snapshot().Count,
                pendingDeliveries=Outbox.Count,auditEvents=Events.Count};
        }
    }
}
