FROM postgres:15.4-bookworm

ARG TARGETPLATFORM=linux/amd64
ARG PGMQ_VER=0.33.1
ARG VECTOR_VER=0.5.1
ARG VECTORIZE_VER=0.5.0
ARG PARTMAN_VER=4.7.3

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

# Install Rust dependencies
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN /root/.cargo/bin/rustup default stable

# install pgvector
RUN git clone https://github.com/pgvector/pgvector.git && \
	cd pgvector && \
	git fetch origin v$VECTOR_VER && \
	git checkout v$VECTOR_VER && \
	make && make install & cd .. && rm -rf pgvector

# install pg_cron
RUN git clone https://github.com/citusdata/pg_cron.git && \
	cd pg_cron && \
	make && make install && cd ../ && rm -rf pg_cron

# install pgmq
RUN /root/.cargo/bin/cargo install cargo-pgrx --version=0.10.2 --locked
RUN /root/.cargo/bin/cargo pgrx init --pg15 $(which pg_config)
RUN git clone https://github.com/tembo-io/pgmq.git && \
	cd pgmq && \
	/root/.cargo/bin/cargo pgrx package --pg-config=$(which pg_config) && \
	cd ../ && rm -rf pgmq

WORKDIR /vectorize

COPY . .

RUN /root/.cargo/bin/cargo install cargo-pgrx --version=0.11.0 --locked
RUN /root/.cargo/bin/cargo pgrx package --pg-config=$(which pg_config)

RUN rm -rf /vectorize

COPY images/vectorize-pg/postgresql.conf /usr/share/postgresql/15/postgresql.conf.sample

USER postgres
CMD ["postgres"]
