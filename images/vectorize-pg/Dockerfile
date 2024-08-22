ARG PG_MAJ=16
FROM postgres:$PG_MAJ-bookworm as builder

ARG VECTOR_VER=0.7.1
ARG VSCALE_VER=0.2.0
ARG CRON_VER=1.6.2
ARG PGRX_VER=0.11.3
ARG PG_MAJ=16

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
	postgresql-server-dev-$PG_MAJ

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

# install pgmq
RUN git clone https://github.com/tembo-io/pgmq.git && \
	cd pgmq/extension && \
	make && \
	make install && \
	cd ../.. && rm -rf pgmq

# Install Rust dependencies
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN $HOME/.cargo/bin/rustup default stable

# install pgrx
RUN $HOME/.cargo/bin/cargo install cargo-pgrx --version=$PGRX_VER --locked
RUN $HOME/.cargo/bin/cargo pgrx init --pg$PG_MAJ $(which pg_config)

# set pgrx to use the correct pg_config
ARG PG_MAJ=16
RUN $HOME/.cargo/bin/cargo pgrx init --pg${PG_MAJ} /usr/bin/pg_config

# install vectorscale
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        export RUSTFLAGS="-C target-feature=+avx2,+fma"; \
    fi

RUN git clone https://github.com/timescale/pgvectorscale.git && \
	cd pgvectorscale/pgvectorscale && \
	git fetch origin $VSCALE_VER && \
	git checkout $VSCALE_VER && \
	sed -i '/blas/d' Cargo.toml && \
	sed -i '/blas_src/d' src/access_method/mod.rs && \
	$HOME/.cargo/bin/cargo pgrx install --pg-config=$(which pg_config)

# install pg-vectorize
COPY . .
RUN cd extension && \
	$HOME/.cargo/bin/cargo pgrx install --pg-config=$(which pg_config)

FROM postgres:$PG_MAJ-bookworm

ARG PG_MAJ=16
COPY --from=builder /usr/share/postgresql/$PG_MAJ/extension /usr/share/postgresql/$PG_MAJ/extension
COPY --from=builder /usr/lib/postgresql/$PG_MAJ/lib /usr/lib/postgresql/$PG_MAJ/lib

RUN apt-get update \
    && apt-get install -y ca-certificates

COPY images/vectorize-pg/postgresql.conf /usr/share/postgresql/$PG_MAJ/postgresql.conf.sample

USER postgres
CMD ["postgres"]