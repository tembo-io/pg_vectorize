FROM ollama/ollama:0.1.32

ARG MODEL=wizardlm2:7b
COPY ./ollama-serve/pull-model.sh .
RUN chmod +x ./pull-model.sh
RUN echo cat
RUN ./pull-model.sh ${MODEL}
