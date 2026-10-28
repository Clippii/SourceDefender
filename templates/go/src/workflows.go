package service

import "sort"

func ReviewBatch(application *Application, session Session, ids []string) ([]string, error) {
    approved := []string{}
    for _, id := range ids {
        record, err := application.Repository.Get(id); if err != nil { return approved, err }
        if record.Status != "pending" { continue }
        if _, err := application.Transition(session, id, "active", record.Version); err != nil { return approved, err }
        approved = append(approved, id)
    }
    return approved, nil
}

func Reconcile(repository *Repository) map[string]interface{} {
    rows := repository.Snapshot(); kinds, states, owners := map[string]int{}, map[string]int{}, map[string]int{}
    for _, row := range rows {
        kinds[row.Kind]++; states[row.Status]++
        if row.Status != "cancelled" { owners[row.Owner] += row.Amount }
    }
    return map[string]interface{}{"record_count": len(rows), "by_kind": kinds, "by_state": states, "amount_by_owner": owners}
}

func PlanRetention(repository *Repository, excludedOwners []string) map[string]interface{} {
    ids := []string{}
    for _, row := range repository.Snapshot() {
        if (row.Status == "archived" || row.Status == "cancelled") && !contains(excludedOwners, row.Owner) { ids = append(ids, row.ID) }
    }
    sort.Strings(ids)
    return map[string]interface{}{"record_ids": ids, "requires_approval": true, "dry_run": true}
}
