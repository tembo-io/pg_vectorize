FROM ollama/ollama:0.1.32

ARG MODEL=mixtral:8x7b
COPY pull-model.sh .
RUN chmod +x ./pull-model.sh
RUN ./pull-model.sh ${MODEL}

