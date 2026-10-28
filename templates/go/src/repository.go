package service

import (
    "errors"
    "sort"
    "sync"
)

var ErrConflict = errors.New("optimistic concurrency conflict")
var ErrMissing = errors.New("record not found")

type Repository struct {
    mu sync.RWMutex
    records map[string]Record
    idempotency map[string]string
}

func NewRepository(records []Record) *Repository {
    r := &Repository{records: map[string]Record{}, idempotency: map[string]string{}}
    for _, record := range records { r.records[record.ID] = record.Copy() }; return r
}

func (r *Repository) Get(id string) (Record, error) {
    r.mu.RLock(); defer r.mu.RUnlock()
    item, ok := r.records[id]
    if !ok { return Record{}, ErrMissing }; return item.Copy(), nil
}

func (r *Repository) Insert(record Record, key string) (Record, error) {
    r.mu.Lock(); defer r.mu.Unlock()
    if previous, ok := r.idempotency[key]; key != "" && ok { return r.records[previous].Copy(), nil }
    if _, exists := r.records[record.ID]; exists { return Record{}, ErrConflict }
    r.records[record.ID] = record.Copy()
    if key != "" { r.idempotency[key] = record.ID }; return record.Copy(), nil
}

func (r *Repository) Save(record Record, expectedVersion int) (Record, error) {
    r.mu.Lock(); defer r.mu.Unlock()
    previous, ok := r.records[record.ID]
    if !ok { return Record{}, ErrMissing }
    if previous.Version != expectedVersion || record.Version != expectedVersion + 1 { return Record{}, ErrConflict }
    r.records[record.ID] = record.Copy(); return record.Copy(), nil
}

type Page struct { Items []Record; Total, Offset, Limit int }
type Filter struct { Owner, Status, Kind string; Offset, Limit int }

func (r *Repository) List(filter Filter) (Page, error) {
    if filter.Offset < 0 || filter.Limit < 1 || filter.Limit > 100 { return Page{}, errors.New("invalid page bounds") }
    rows := []Record{}
    for _, record := range r.Snapshot() {
        if (filter.Owner == "" || filter.Owner == record.Owner) &&
            (filter.Status == "" || filter.Status == record.Status) &&
            (filter.Kind == "" || filter.Kind == record.Kind) { rows = append(rows, record) }
    }
    sort.Slice(rows, func(i, j int) bool { return rows[i].ID < rows[j].ID })
    start := filter.Offset; if start > len(rows) { start = len(rows) }
    end := start + filter.Limit; if end > len(rows) { end = len(rows) }
    return Page{rows[start:end], len(rows), filter.Offset, filter.Limit}, nil
}

func (r *Repository) Snapshot() []Record {
    r.mu.RLock(); defer r.mu.RUnlock()
    rows := make([]Record, 0, len(r.records))
    for _, record := range r.records { rows = append(rows, record.Copy()) }; return rows
}
