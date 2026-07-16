package cmd

import (
	"context"
	stdlog "log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/brdgme/brdgme/brdgme-go/brdgme"
)

// Serve starts an HTTP server which handles requests by calling Cli with a
// fresh game value from newGame for each request, matching the Rust HTTP
// server's per-request GameRequester behaviour. Listens on the address in
// the ADDR env var, defaulting to 0.0.0.0:8080. Shuts down gracefully on
// SIGINT/SIGTERM.
func Serve(newGame func() brdgme.Gamer) {
	addr := os.Getenv("ADDR")
	if addr == "" {
		addr = "0.0.0.0:8080"
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		Cli(newGame(), r.Body, w)
	})

	server := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sig
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := server.Shutdown(ctx); err != nil {
			stdlog.Printf("graceful shutdown failed: %v", err)
		}
	}()

	stdlog.Printf("listening on %s", addr)
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		stdlog.Fatalf("server error: %v", err)
	}
}
