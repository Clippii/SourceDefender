package service

import "testing"

func sampleRecord() Record {
    return Record{ID:"rec-001",Kind:"release",Owner:"user-1",Title:"Example record",Amount:1250,Status:"pending",Version:1,Tags:[]string{}}
}

func TestRepositoryInsertAndSnapshot(t *testing.T) {
    repo := NewRepository(nil)
    if _,err := repo.Insert(sampleRecord(),"");err != nil { t.Fatal(err) }
    row,err := repo.Get("rec-001");if err != nil || row.Amount != 1250 { t.Fatal("read mismatch") }
    row.Title = "Changed";again,_ := repo.Get("rec-001")
    if again.Title != "Example record" { t.Fatal("read mutated storage") }
}

func TestIdempotency(t *testing.T) {
    repo := NewRepository(nil);repo.Insert(sampleRecord(),"key")
    second := sampleRecord();second.ID = "rec-002"
    again,err := repo.Insert(second,"key")
    if err != nil || again.ID != "rec-001" || len(repo.Snapshot()) != 1 { t.Fatal("duplicate request") }
    if _,err := repo.Insert(sampleRecord(),"");err != ErrConflict { t.Fatal("duplicate identity accepted") }
}

func TestOptimisticConflict(t *testing.T) {
    repo := NewRepository([]Record{sampleRecord()});changed,_ := sampleRecord().Transition("active")
    if _,err := repo.Save(changed,1);err != nil { t.Fatal(err) }
    if _,err := repo.Save(changed,1);err != ErrConflict { t.Fatal("stale write accepted") }
}

func TestPageBoundsAndTransitions(t *testing.T) {
    repo := NewRepository([]Record{sampleRecord()})
    if _,err := repo.List(Filter{Limit:0});err == nil { t.Fatal("invalid limit accepted") }
    page,err := repo.List(Filter{Owner:"user-1",Limit:1})
    if err != nil || page.Total != 1 { t.Fatal("owner filter failed") }
    if _,err := sampleRecord().Transition("archived");err == nil { t.Fatal("skipped review") }
}
