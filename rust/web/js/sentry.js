import * as Sentry from "@sentry/browser";
import { wasmIntegration } from "@sentry/wasm";

// No Sentry.init() here: the real DSN/release are only known at SSR render
// time (see app.rs's shell()), which emits a separate inline <script> that
// calls window.Sentry.init(...) after this bundle has loaded.
window.Sentry = Sentry;
window.SentryWasmIntegration = wasmIntegration;
