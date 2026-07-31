FROM gcr.io/distroless/cc-debian12@sha256:7ee09f36862efbdbf70422db263e411c2618409ca46faa555bd5b636155307df AS foo-1
COPY --from=builder /app/target/release/foo_1_http .
USER 65532
CMD ["./foo_1_http"]
