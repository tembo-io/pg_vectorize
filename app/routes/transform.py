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

if TYPE_CHECKING:
    Vector = List[str]
else:
    Vector = conlist(str, min_length=1)

class Batch(BaseModel):
    prompts: Vector

BATCH_SIZE = os.getenv("BATCH_SIZE", 1000)

@router.post("/transform", response_model=None)
def batch_transform(payload: Batch) -> np.array:
    logging.info({"batch-predict-len": len(payload.prompts)})
    batches = chunk_list(payload.prompts, BATCH_SIZE)
    num_batches = len(batches)
    responses: list[float] = []
    for i, batch in enumerate(batches):
        logging.info(f"Batch {i} / {num_batches}")
        responses.extend(model.encode(batch).tolist())
    print(responses)
    return responses


def chunk_list(lst: List[Any], chunk_size: int) -> List[List[Any]]:
    """Split a list into smaller lists of equal length, except the last one."""
    chunks = []
    for i in range(0, len(lst), chunk_size):
        chunks.append(lst[i : i + chunk_size])
    return chunks
