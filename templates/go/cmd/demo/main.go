package main

import (
    "encoding/json"
    "fmt"
    "os"
    service "exercise.local/service/src"
)

func main() {
    app, err := service.BuildApplication(".")
    if err != nil { fmt.Fprintln(os.Stderr, err); os.Exit(1) }
    command := "health"; if len(os.Args) > 1 { command = os.Args[1] }
    var result interface{}
    switch command {
    case "health": result = app.Health()
    case "list": result, err = app.List(service.ExampleSession(), 0, 20)
    case "export": result, err = app.Export(service.ExampleSession())
    case "reconcile": result = service.Reconcile(app.Repository)
    default: err = fmt.Errorf("usage: go run ./cmd/demo health|list|export|reconcile")
    }
    if err != nil { fmt.Fprintln(os.Stderr, err); os.Exit(1) }
    body, err := json.MarshalIndent(result, "", "  ")
    if err != nil { fmt.Fprintln(os.Stderr, err); os.Exit(1) }; fmt.Println(string(body))
}
