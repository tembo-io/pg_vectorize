import logging
import os
from typing import TYPE_CHECKING, Any, List

from fastapi import APIRouter
from pydantic import BaseModel, conlist
import numpy as np

router = APIRouter(tags=["transform"])

logging.basicConfig(level=logging.DEBUG)

from sentence_transformers import SentenceTransformer

model = SentenceTransformer("./models")
model_name = "all-MiniLM-L12-v2"

if TYPE_CHECKING:
    Vector = List[str]
else:
    Vector = conlist(str, min_length=1)


class Batch(BaseModel):
    input: Vector
    model: str = model_name

BATCH_SIZE = os.getenv("BATCH_SIZE", 1000)


class ResponseModel(BaseModel):
    data: list[list[float]]
    model: str = model_name


@router.post("/v1/embeddings", response_model=ResponseModel)
def batch_transform(payload: Batch) -> ResponseModel:
    logging.info({"batch-predict-len": len(payload.input)})
    batches = chunk_list(payload.input, BATCH_SIZE)
    num_batches = len(batches)
    responses: list[list[float]] = []
    for i, batch in enumerate(batches):
        logging.info(f"Batch {i} / {num_batches}")
        responses.extend(model.encode(batch).tolist())
    logging.info("Completed %s batches", num_batches)
    return ResponseModel(
        data=responses,
        model=model_name,
    )


def chunk_list(lst: List[Any], chunk_size: int) -> List[List[Any]]:
    """Split a list into smaller lists of equal length, except the last one."""
    chunks = []
    for i in range(0, len(lst), chunk_size):
        chunks.append(lst[i : i + chunk_size])
    return chunks
