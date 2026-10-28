using System;
using System.Linq;
using System.Collections.Generic;

namespace Service {
    public static class Workflows {
        public static List<string> ReviewBatch(Application application,Session session,IEnumerable<string> ids) {
            var approved=new List<string>();
            foreach(string id in ids) {
                var record=application.Repository.Get(id);
                if(record.status!="pending")continue;
                application.Transition(session,id,"active",record.version);approved.Add(id);
            }
            return approved;
        }

        public static object Reconcile(Repository repository) {
            var records=repository.Snapshot();
            return new {recordCount=records.Count,
                byKind=records.GroupBy(r=>r.kind).ToDictionary(g=>g.Key,g=>g.Count()),
                byState=records.GroupBy(r=>r.status).ToDictionary(g=>g.Key,g=>g.Count()),
                amountByOwner=records.Where(r=>r.status!="cancelled").GroupBy(r=>r.owner)
                    .ToDictionary(g=>g.Key,g=>g.Sum(r=>r.amount))};
        }

        public static object PlanRetention(Repository repository,IEnumerable<string> excludedOwners) {
            var excluded=new HashSet<string>(excludedOwners);
            return new {recordIds=repository.Snapshot().Where(r=>(r.status=="archived"||r.status=="cancelled")
                &&!excluded.Contains(r.owner)).Select(r=>r.id).OrderBy(x=>x).ToArray(),requiresApproval=true,dryRun=true};
        }
    }
}
