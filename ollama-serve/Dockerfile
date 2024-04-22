FROM ollama/ollama:0.1.32

ARG MODEL=wizardlm2:7b-q2_K
COPY ./ollama-serve/pull-model.sh .
RUN chmod +x ./pull-model.sh
RUN ./pull-model.sh ${MODEL}
