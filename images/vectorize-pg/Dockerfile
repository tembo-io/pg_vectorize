FROM postgres:16.2-bookworm as builder

ARG PGMQ_VER=1.1.1
ARG VECTOR_VER=0.6.0
ARG CRON_VER=1.6.2
ARG PGRX_VER=0.11.3

RUN apt-get update \
    && apt-get install -y \
	ca-certificates \
	clang \
	curl \
	gcc \
	git \
	libssl-dev \
	make \
	pkg-config \
	postgresql-server-dev-15

WORKDIR /vectorize

# install pgvector
RUN git clone https://github.com/pgvector/pgvector.git && \
	cd pgvector && \
	git fetch origin v$VECTOR_VER && \
	git checkout v$VECTOR_VER && \
	make && make install && \
    cd .. && rm -rf pgvector

# install pg_cron
RUN git clone https://github.com/citusdata/pg_cron.git --branch v$CRON_VER && \
	cd pg_cron && \
	make && make install && \
    cd .. && rm -rf pg_cron

# Install Rust dependencies
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN $HOME/.cargo/bin/rustup default stable

# install pgrx
RUN $HOME/.cargo/bin/cargo install cargo-pgrx --version=$PGRX_VER --locked

# install pgmq
RUN $HOME/.cargo/bin/cargo pgrx init --pg15 $(which pg_config)

RUN git clone https://github.com/tembo-io/pgmq.git && \
    cd pgmq && $HOME/.cargo/bin/cargo pgrx install --pg-config=$(which pg_config) && \
    cd .. && rm -rf pgmq

# install pg-vectorize
COPY . .
RUN cd extension && \
	$HOME/.cargo/bin/cargo pgrx install --pg-config=$(which pg_config)

FROM postgres:16.2-bookworm

COPY --from=builder /usr/share/postgresql/15/extension /usr/share/postgresql/15/extension
COPY --from=builder /usr/lib/postgresql/15/lib /usr/lib/postgresql/15/lib

RUN apt-get update \
    && apt-get install -y ca-certificates

COPY images/vectorize-pg/postgresql.conf /usr/share/postgresql/15/postgresql.conf.sample

USER postgres
CMD ["postgres"]