FROM rust:1.95-bookworm

WORKDIR /work
COPY . .

RUN cargo test
RUN for src in demos/*.c; do \
      asm="/tmp/$(basename "$src" .c).s"; \
      cargo run --quiet -- cc "$src" -o "$asm"; \
      cargo run --quiet -- run "$asm"; \
    done

CMD ["bash", "-lc", "cargo test && for src in demos/*.c; do asm=\"/tmp/$(basename \"$src\" .c).s\"; cargo run --quiet -- cc \"$src\" -o \"$asm\"; echo \"== $src ==\"; cargo run --quiet -- run \"$asm\"; done"]
