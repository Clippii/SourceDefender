package service

import (
    "encoding/json"
    "os"
    "path/filepath"
)

func BuildApplication(root string) (*Application, error) {
    recordsBody, err := os.ReadFile(filepath.Join(root, "fixtures", "records.json"))
    if err != nil { return nil, err }
    settingsBody, err := os.ReadFile(filepath.Join(root, "config", "scenario.json"))
    if err != nil { return nil, err }
    var records []Record; var settings Settings
    if err := json.Unmarshal(recordsBody, &records); err != nil { return nil, err }
    if err := json.Unmarshal(settingsBody, &settings); err != nil { return nil, err }
    return NewApplication(NewRepository(records), settings), nil
}

func ExampleSession() Session {
    return Session{ID: "user-1", Token: "fixture-session", Verified: true, Roles: []string{"operator", "reviewer"}}
}
