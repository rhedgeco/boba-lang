test *args:
    cargo nextest run {{ args }}

[no-cd]
lex target:
    cargo run -p boba-analyzer -- lex {{ target }}
