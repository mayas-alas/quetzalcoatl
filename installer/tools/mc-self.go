// GnX source-build addition to the pinned MinIO CLI (AGPL-3.0).
// Resolve the parent account in memory, never exposing its access ID in OS argv.
// This does not change the upstream API, credential policy or secret key handling.
package main

import (
    "encoding/json"
    "os"
    "path/filepath"
)

func init() {
    position := -1
    directory := ""
    for i, argument := range os.Args {
        if argument == "@self" { position = i }
        if argument == "--config-dir" && i+1 < len(os.Args) { directory = os.Args[i+1] }
    }
    if position < 0 { return }
    if position < 1 || directory == "" { os.Exit(64) }
    path := filepath.Join(directory, "config.json")
    info, err := os.Lstat(path)
    if err != nil || !info.Mode().IsRegular() || info.Mode().Perm() != 0600 { os.Exit(65) }
    data, err := os.ReadFile(path)
    if err != nil { os.Exit(65) }
    defer func() { for i := range data { data[i] = 0 } }()
    var config struct { Aliases map[string]struct { AccessKey string `json:"accessKey"` } `json:"aliases"` }
    if json.Unmarshal(data, &config) != nil { os.Exit(65) }
    alias := os.Args[position-1]
    account, present := config.Aliases[alias]
    if !present || account.AccessKey == "" { os.Exit(65) }
    os.Args[position] = account.AccessKey
}
