# Dev Networking Constraints

## Hybrid Dev Networking

Local web server cannot resolve `*.svc.cluster.local`. mirrord wraps `cargo leptos watch` in the Tiltfile targeting `pod/postgres-0`.

On NixOS, `/etc/hosts` is read-only - kubefwd is not viable as an alternative.

## CRD Startup Gate

The `crd-ready` Tilt resource uses `kubectl wait --for=condition=established` to gate the operator. Without this the operator fails with "event queue error" on startup while the API server registers the CRD.
