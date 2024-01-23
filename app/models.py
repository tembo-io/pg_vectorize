import logging

from fastapi import FastAPI

from sentence_transformers import SentenceTransformer

_HF_ORG = "sentence-transformers"

MODELS_TO_CACHE = [f"{_HF_ORG}/all-MiniLM-L12-v2"]

cache_dir = "./models"


def load_model_cache(app: FastAPI) -> dict[str, SentenceTransformer]:
    model_cache = {}
    for m in MODELS_TO_CACHE:
        saved_path = _model_dir(m)
        model_cache[m] = SentenceTransformer(saved_path)
    app.state.model_cache = model_cache


def save_model_cache() -> None:
    """caches models to local storage"""
    for mod in MODELS_TO_CACHE:
        logging.debug(f"Caching model: {mod}")
        model = SentenceTransformer(mod)
        save_dir = _model_dir(mod)
        model.save(save_dir)


def _model_dir(model: str) -> str:
    model_dir = model.replace("/", "_")
    return f"{cache_dir}/{model_dir}"


def model_name(model: str) -> str:
    """prepends with the HF if the org is not specified"""
    if "/" not in model_name:
        return f"{_HF_ORG}/{model_name}"


if __name__ == "__main__":
    save_model_cache()
