package service

import (
    "errors"
    "strings"
    "time"
)

type Principal struct {
    ID string `json:"id"`
    Roles []string `json:"roles"`
    Verified bool `json:"verified"`
}

func (p Principal) Can(role string) bool { return p.Verified && contains(p.Roles, role) }

type Session struct {
    ID string
    Token string
    Verified bool
    Roles []string
}

type Record struct {
    ID string `json:"id"`
    Kind string `json:"kind"`
    Owner string `json:"owner"`
    Title string `json:"title"`
    Status string `json:"status"`
    Version int `json:"version"`
    Amount int `json:"amount"`
    Tags []string `json:"tags"`
}

func (r Record) Copy() Record { r.Tags = append([]string{}, r.Tags...); return r }

func (r Record) Transition(next string) (Record, error) {
    allowed := map[string][]string{
        "pending": {"active", "cancelled"}, "active": {"archived", "cancelled"},
        "archived": {}, "cancelled": {},
    }
    if !contains(allowed[r.Status], next) { return Record{}, errors.New("invalid record transition") }
    r = r.Copy(); r.Status = next; r.Version++
    return r, nil
}

type Event struct { Action, Actor, RecordID, Timestamp string }
func newEvent(action, actor, recordID string) Event {
    return Event{action, actor, recordID, time.Now().UTC().Format(time.RFC3339)}
}

type Settings struct {
    Name string `json:"name"`
    Slug string `json:"slug"`
    RecordKinds []string `json:"record_kinds"`
    Workflows []string `json:"workflows"`
    PageSize int `json:"page_size"`
    MaxExportRows int `json:"max_export_rows"`
    RetentionDays int `json:"retention_days"`
}

func contains(values []string, value string) bool {
    for _, candidate := range values { if candidate == value { return true } }; return false
}
func suffix(value, prefix string) (string, bool) {
    if !strings.HasPrefix(value, prefix) { return "", false }; return strings.TrimPrefix(value, prefix), true
}
