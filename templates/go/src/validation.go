package service

import (
    "errors"
    "regexp"
    "sort"
    "strings"
)

var identifierPattern = regexp.MustCompile(`^[a-z][a-z0-9-]{1,63}$`)

func ValidateRecord(record Record, settings Settings) (Record, error) {
    if !identifierPattern.MatchString(record.ID) || !identifierPattern.MatchString(record.Owner) {
        return Record{}, errors.New("expected lowercase record and owner identifiers")
    }
    if !contains(settings.RecordKinds, record.Kind) { return Record{}, errors.New("unsupported record kind") }
    record.Title = strings.Join(strings.Fields(record.Title), " ")
    if len(record.Title) < 3 || len(record.Title) > 160 { return Record{}, errors.New("title length out of range") }
    if record.Amount < 0 || record.Amount > 100000000 { return Record{}, errors.New("amount out of range") }
    if len(record.Tags) > 10 { return Record{}, errors.New("too many tags") }
    seen := map[string]bool{}; tags := []string{}
    for _, tag := range record.Tags {
        if !identifierPattern.MatchString(tag) { return Record{}, errors.New("invalid tag") }
        if !seen[tag] { seen[tag] = true; tags = append(tags, tag) }
    }
    sort.Strings(tags); record.Tags = tags
    record.Status = "pending"; record.Version = 1
    return record, nil
}
