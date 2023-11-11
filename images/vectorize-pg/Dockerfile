FROM rust:1.70-bookworm as builder

ARG TRUNK_VER=0.11.6

RUN cargo install --version $TRUNK_VER pg-trunk

FROM postgres:15.4-bookworm

COPY --from=builder /usr/local/cargo/bin/trunk /usr/bin/trunk

ARG PGMQ_VER=0.33.1
ARG VECTORIZE_VER=0.5.0
ARG PARTMAN_VER=4.7.3

RUN apt-get update \
    && apt-get install -y ca-certificates

RUN trunk install pg_partman --version $PARTMAN_VER
RUN trunk install pgmq --version $PGMQ_VER
RUN trunk install vectorize --version $VECTORIZE_VER

COPY ./postgresql.conf /usr/share/postgresql/15/postgresql.conf.sample

USER postgres
CMD ["postgres"]
