import logging
import os
from typing import TYPE_CHECKING, Any, List

from app.models import model_org_name, get_model, parse_header
from fastapi import APIRouter, Header, HTTPException, Request
from pydantic import BaseModel, conlist

# Chunking functions
def chunk_text(text, max_length):
    """Splits text into smaller chunks based on a maximum character length."""
    import re
    sentences = re.split(r'(?<=[.!?])\s+', text)  # Split by sentence or paragraph boundaries
    chunks = []
    current_chunk = ""
    
    for sentence in sentences:
        if len(current_chunk) + len(sentence) <= max_length:
            current_chunk += sentence + " "
        else:
            chunks.append(current_chunk.strip())
            current_chunk = sentence + " "
    if current_chunk:
        chunks.append(current_chunk.strip())
    
    return chunks

def chunk_table(input_data, max_length):
    """Chunk each item in a list of texts into smaller pieces."""
    chunked_data = []
    for text in input_data:
        chunks = chunk_text(text, max_length)
        chunked_data.extend(chunks)
    return chunked_data


router = APIRouter(tags=["transform"])

logging.basicConfig(level=logging.DEBUG)

BATCH_SIZE = int(os.getenv("BATCH_SIZE", 1000))


if TYPE_CHECKING:
    Vector = List[str]
else:
    Vector = conlist(str, min_length=1)


class Batch(BaseModel):
    input: Vector
    model: str = "all-MiniLM-L6-v2"
    normalize: bool = False


class Embedding(BaseModel):
    embedding: list[float]
    index: int


class ResponseModel(BaseModel):
    data: list[Embedding]
    model: str


@router.post("/v1/embeddings", response_model=ResponseModel)
def batch_transform(
    request: Request, payload: Batch, authorization: str = Header(None)
) -> ResponseModel:
    logging.info({"batch-predict-len": len(payload.input)})
    
    # Preprocess by chunking large texts in payload.input
    chunked_input = chunk_table(payload.input, max_length=500)  # You can adjust the max_length as needed
    batches = chunk_list(chunked_input, BATCH_SIZE)
    
    num_batches = len(batches)
    responses: list[list[float]] = []

    requested_model = model_org_name(payload.model)

    api_key = parse_header(authorization)    
    try:
        model = get_model(
            model_name=requested_model,
            model_cache=request.app.state.model_cache,
            api_key=api_key,
        )
    except Exception as e:
        raise HTTPException(
            status_code=400,
            detail=f"Unable to load {payload.model} -- {e}",
        )

    for idx, batch in enumerate(batches):
        logging.info(f"Batch {idx} / {num_batches}")
        responses.extend(
            model.encode(
                sentences=batch, normalize_embeddings=payload.normalize
            ).tolist()
        )
    logging.info("Completed %s batches", num_batches)
    embeds = [
        Embedding(embedding=embedding, index=i) for i, embedding in enumerate(responses)
    ]
    return ResponseModel(
        data=embeds,
        model=requested_model,
    )


def chunk_list(lst: List[Any], chunk_size: int) -> List[List[Any]]:
    """Split a list into smaller lists of equal length, except the last one."""
    chunks = []
    for i in range(0, len(lst), chunk_size):
        chunks.append(lst[i : i + chunk_size])
    return chunks
