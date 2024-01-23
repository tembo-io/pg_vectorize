import logging
import os
from typing import TYPE_CHECKING, Any, List

from app.models import model_name
from fastapi import APIRouter, HTTPException, Request
from pydantic import BaseModel, conlist

router = APIRouter(tags=["transform"])

logging.basicConfig(level=logging.DEBUG)

from sentence_transformers import SentenceTransformer

try:
    MULTI_MODEL = int(os.getenv("MULTI_MODEL", 1))
except Exception:
    MULTI_MODEL = 1

BATCH_SIZE = int(os.getenv("BATCH_SIZE", 1000))


if TYPE_CHECKING:
    Vector = List[str]
else:
    Vector = conlist(str, min_length=1)


class Batch(BaseModel):
    input: Vector
    model: str = "all-MiniLM-L12-v2"


class Embedding(BaseModel):
    embedding: list[float]
    index: int


class ResponseModel(BaseModel):
    data: list[Embedding]
    model: str


@router.post("/v1/embeddings", response_model=ResponseModel)
def batch_transform(request: Request, payload: Batch) -> ResponseModel:
    logging.info({"batch-predict-len": len(payload.input)})
    batches = chunk_list(payload.input, BATCH_SIZE)
    num_batches = len(batches)
    responses: list[list[float]] = []

    requested_model = model_name(payload.model)

    try:
        model = get_model(
            model_name=payload.model, model_cache=request.app.state.model_cache
        )
    except Exception as e:
        raise HTTPException(
            status_code=400,
            detail=f"Unable to load {payload.model} -- {e}",
        )

    for idx, batch in enumerate(batches):
        logging.info(f"Batch {idx} / {num_batches}")
        responses.extend(model.encode(batch).tolist())
    logging.info("Completed %s batches", num_batches)
    embeds = [
        Embedding(embedding=embedding, index=i) for i, embedding in enumerate(responses)
    ]
    return ResponseModel(
        data=embeds,
        model=payload.model,
    )


def chunk_list(lst: List[Any], chunk_size: int) -> List[List[Any]]:
    """Split a list into smaller lists of equal length, except the last one."""
    chunks = []
    for i in range(0, len(lst), chunk_size):
        chunks.append(lst[i : i + chunk_size])
    return chunks


def get_model(
    model_name: str, model_cache: dict[str, SentenceTransformer]
) -> SentenceTransformer:
    if MULTI_MODEL:
        model = model_cache.get(model_name)
    else:
        raise HTTPException(
            status_code=400,
            detail="Must enable multi-model via MULTI_MODEL env var",
        )
    if model is None:
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
