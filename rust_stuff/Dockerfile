FROM rust:latest

WORKDIR /app

RUN apt-get update &&  \
    apt-get install -y wget unzip && \
    apt-get install git-lfs


RUN wget https://download.pytorch.org/libtorch/cu118/libtorch-cxx11-abi-shared-with-deps-2.0.0%2Bcu118.zip
RUN unzip libtorch-cxx11-abi-shared-with-deps-2.0.0+cu118.zip -d /opt

ENV LIBTORCH=/opt/libtorch

RUN git lfs install
RUN mkdir resources
RUN git -C resources clone https://huggingface.co/sentence-transformers/all-MiniLM-L12-v2

ENV LD_LIBRARY_PATH=${LIBTORCH}/lib:$LD_LIBRARY_PATH

COPY . .

RUN cargo build --release
EXPOSE 8080
CMD ["./target/release/vector-serve"]