test *args:
    cargo nextest run {{ args }}

lex target:
    cargo run -p boba-analyzer -- lex {{ target }}
