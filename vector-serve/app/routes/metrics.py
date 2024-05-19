from typing import Callable
from prometheus_fastapi_instrumentator.metrics import Info
from prometheus_client import Counter

def http_requested_models_total() -> Callable[[Info], None]:
    METRIC = Counter(
        "http_requested_model",
        "Number of times a certain model has been requested.",
        labelnames=("models",)
    )

    def instrumentation(info: Info) -> None:
        langs = set()
        lang_str = info.request.headers["Accept-Language"]
        for element in lang_str.split(","):
            element = element.split(";")[0].strip().lower()
            langs.add(element)
        for language in langs:
            METRIC.labels(language).inc()

    return instrumentation
