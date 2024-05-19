from prometheus_client import Counter


ML_MODEL_COUNT = Counter(
    "http_requested_model",
    "Number of times a certain model has been requested.",
    ["model_name"],
)
