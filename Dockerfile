FROM rust:1.95-bookworm

WORKDIR /work
COPY . .

RUN cargo test
RUN bash scripts/run_demos.sh
RUN bash scripts/run_sbase.sh

CMD ["bash", "-c", "cargo test && bash scripts/run_demos.sh && bash scripts/run_sbase.sh"]
