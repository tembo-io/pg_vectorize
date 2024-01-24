import os
import logging

from fastapi import FastAPI, HTTPException

from sentence_transformers import SentenceTransformer

_HF_ORG = "sentence-transformers"

MODELS_TO_CACHE = [f"{_HF_ORG}/all-MiniLM-L12-v2"]

cache_dir = "./models"

try:
    MULTI_MODEL = int(os.getenv("MULTI_MODEL", 1))
except Exception:
    MULTI_MODEL = 1


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


def model_org_name(model_name: str) -> str:
    """prepends with the HF if the org is not specified"""
    if "/" not in model_name:
        return f"{_HF_ORG}/{model_name}"
    else:
        return model_name


def get_model(
    model_name: str, model_cache: dict[str, SentenceTransformer]
) -> SentenceTransformer:

    model = model_cache.get(model_name)
    if model is None:
        if not MULTI_MODEL:
            raise HTTPException(
                status_code=400,
                detail="Must enable multi-model via MULTI_MODEL env var",
            )
        # try to download from HF when MULTI_MODEL enabled
        # and model not in cache
        logging.debug(f"Model: {model_name} not in cache.")
        try:
            model = SentenceTransformer(model_name)
            # add model to cache
            model_cache[model_name] = model
            logging.debug(f"Added model: {model_name} to cache.")
        except Exception:
            logging.exception("Failed to load model %s", model_name)
            raise
    return model


if __name__ == "__main__":
    save_model_cache()
