package service

import (
    "testing"
    "time"
)

func testApp() *Application {
    return NewApplication(NewRepository(nil),Settings{RecordKinds:[]string{"release"},MaxExportRows:1000})
}

func TestCreateAndReview(t *testing.T) {
    app := testApp();session := ExampleSession()
    if _,err := app.Create(session,sampleRecord(),"");err != nil { t.Fatal(err) }
    changed,err := app.Transition(session,"rec-001","active",1)
    if err != nil || changed.Version != 2 || len(app.Events) != 2 { t.Fatal("lifecycle mismatch",err) }
}

func TestCreationBoundaries(t *testing.T) {
    app := testApp();session := ExampleSession();session.Roles = []string{"viewer"}
    if _,err := app.Create(session,sampleRecord(),"");err == nil { t.Fatal("viewer created record") }
    row := sampleRecord();row.Owner = "user-2"
    if _,err := app.Create(ExampleSession(),row,"");err == nil { t.Fatal("cross-owner write") }
    row = sampleRecord();row.Kind = "unknown"
    if _,err := app.Create(ExampleSession(),row,"");err == nil { t.Fatal("unknown domain") }
}

func TestRouterUnknownOperation(t *testing.T) {
    router := Router{Application:testApp()}
    if router.Dispatch(Request{Operation:"unknown"}).Status != 404 { t.Fatal("wrong status") }
}

func TestReviewWorkflow(t *testing.T) {
    app := testApp();app.Create(ExampleSession(),sampleRecord(),"")
    approved,err := ReviewBatch(app,ExampleSession(),[]string{"rec-001"})
    if err != nil || len(approved) != 1 { t.Fatal("review failed",err) }
    if Reconcile(app.Repository)["record_count"] != 1 { t.Fatal("reconciliation mismatch") }
}

func TestQueueRetry(t *testing.T) {
    queue := NewJobQueue(2);now := time.Now()
    if !queue.Enqueue(&Job{ID:"one",Kind:"backup"}) || queue.Enqueue(&Job{ID:"one",Kind:"backup"}) { t.Fatal("deduplication failed") }
    handlers := map[string]func(map[string]interface{})error{"backup":func(map[string]interface{})error{return invalid("fixture failure")}}
    if queue.RunOnce(handlers,now).State != "retry" { t.Fatal("no retry") }
    if queue.RunOnce(handlers,now) != nil { t.Fatal("retried too soon") }
    if queue.RunOnce(handlers,now.Add(10*time.Second)).State != "failed" { t.Fatal("missing dead letter") }
}
