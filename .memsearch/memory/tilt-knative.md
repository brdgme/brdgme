# Tilt + Knative Integration

## Official Tilt Knative Extension

https://github.com/tilt-dev/tilt-extensions/blob/master/knative/Tiltfile

Key functions:
- `knative_install(version)`: installs Knative CRDs + core, configures dev registries (`kind.local`, `ko.local`, `dev.local`), waits for webhook readiness
- `knative_yaml(file)`: processes Knative Service resources; auto-injects `autoscaling.knative.dev/minScale: "1"` to prevent scale-to-zero killing Tilt's live update target

The extension does NOT handle networking/ingress - that is configured separately.

## Critical: minScale Must Be 1 in Dev

Always set `minScale: 1` on all dev Knative Services. Without it Tilt's live update target pod is killed when traffic goes to zero.

The brdgme Tiltfile already sets `minScale: 1` manually via annotations on each Knative Service. The official extension could replace that boilerplate, but the manual approach also works.
