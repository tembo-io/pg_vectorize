ARG PG_MAJ=16
FROM postgres:$PG_MAJ-bookworm as builder

ARG VECTOR_VER=0.7.4
ARG VSCALE_VER=0.3.0
ARG CRON_VER=1.6.4
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

# Install Rust dependencies
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN $HOME/.cargo/bin/rustup default stable

# install pgrx
RUN $HOME/.cargo/bin/cargo install cargo-pgrx --version=$PGRX_VER --locked
RUN $HOME/.cargo/bin/cargo pgrx init --pg$PG_MAJ $(which pg_config)

# set pgrx to use the correct pg_config
ARG PG_MAJ=16
RUN $HOME/.cargo/bin/cargo pgrx init --pg${PG_MAJ} /usr/bin/pg_config

COPY . .

# install dependencies
ARG PG_MAJ=16
RUN export PATH="${HOME}/.cargo/bin:$PATH" && cd extension && make setup.dependencies PGRX_PG_CONFIG=$(cargo pgrx info pg-config pg${PG_MAJ})

# install pg-vectorize
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