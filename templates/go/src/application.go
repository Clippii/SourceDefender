package service

import "errors"

type Application struct {
    Repository *Repository
    Settings Settings
    Events []Event
    Outbox []Delivery
}

func NewApplication(repository *Repository, settings Settings) *Application {
    return &Application{Repository: repository, Settings: settings, Events: []Event{}, Outbox: []Delivery{}}
}

func (a *Application) principal(session Session) (Principal, error) {
    user, ok := Authenticate(session)
    if !ok { return Principal{}, errors.New("verified session required") }
    return EffectivePrincipal(user), nil
}

func (a *Application) Create(session Session, input Record, key string) (Record, error) {
    user, err := a.principal(session); if err != nil { return Record{}, err }
    if !user.Can("operator") { return Record{}, errors.New("operator role required") }
    record, err := ValidateRecord(input, a.Settings); if err != nil { return Record{}, err }
    if record.Owner != user.ID { return Record{}, errors.New("owner mismatch") }
    saved, err := a.Repository.Insert(record, key)
    if err == nil { AppendEvent(&a.Events, newEvent("record.created", user.ID, saved.ID)) }
    return saved, err
}

func (a *Application) List(session Session, offset, limit int) (Page, error) {
    user, err := a.principal(session); if err != nil { return Page{}, err }
    owner := user.ID; if user.Can("reviewer") { owner = "" }
    return a.Repository.List(Filter{Owner: owner, Offset: offset, Limit: limit})
}

func (a *Application) Transition(session Session, id, target string, version int) (Record, error) {
    user, err := a.principal(session); if err != nil { return Record{}, err }
    if !user.Can("reviewer") { return Record{}, errors.New("reviewer role required") }
    record, err := a.Repository.Get(id); if err != nil { return Record{}, err }
    changed, err := record.Transition(target); if err != nil { return Record{}, err }
    saved, err := a.Repository.Save(changed, version)
    if err == nil { AppendEvent(&a.Events, newEvent("record." + target, user.ID, id)) }; return saved, err
}

func (a *Application) Invoice(session Session, id string) (Invoice, error) {
    user, err := a.principal(session); if err != nil { return Invoice{}, err }
    record, err := a.Repository.Get(id); if err != nil { return Invoice{}, err }
    if record.Owner != user.ID && !user.Can("reviewer") { return Invoice{}, errors.New("owner mismatch") }
    return BuildInvoice(record, 2000)
}

func (a *Application) Export(session Session) (ExportPlan, error) {
    user, err := a.principal(session); if err != nil { return ExportPlan{}, err }
    rows := []Record{}
    for _, record := range a.Repository.Snapshot() { if record.Owner == user.ID { rows = append(rows, record) } }
    plan, err := PlanExport(rows, user.ID, a.Settings.MaxExportRows)
    if err == nil { a.Outbox = append(a.Outbox, plan.Deliveries...) }; return plan, err
}

func (a *Application) Health() map[string]interface{} {
    return map[string]interface{}{"status": "ready", "records": len(a.Repository.Snapshot()),
        "pending_deliveries": len(a.Outbox), "audit_events": len(a.Events)}
}
