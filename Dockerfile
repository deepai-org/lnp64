FROM rust:1.95-bookworm

WORKDIR /work
COPY . .

RUN cargo test
RUN bash scripts/run_demos.sh

CMD ["bash", "-c", "cargo test && bash scripts/run_demos.sh"]
