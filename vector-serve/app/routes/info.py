from fastapi import APIRouter, HTTPException, Request, Query
from pydantic import BaseModel
from sentence_transformers import SentenceTransformer
import logging

from app.models import model_org_name, get_model

router = APIRouter(tags=["info"])


class InfoResponse(BaseModel):
    model: str
    max_seq_len: int
    embedding_dimension: int


@router.get("/v1/info/", response_model=InfoResponse)
def model_info(request: Request, model_name: str = Query(...)) -> InfoResponse:
    requested_model = model_org_name(model_name)
    try:
        model: SentenceTransformer = get_model(
            model_name=requested_model, model_cache=request.app.state.model_cache
        )
    except Exception as e:
        raise HTTPException(
            status_code=400,
            detail=f"Unable to load {requested_model} -- {e}",
        )
    logging.debug(requested_model)
    logging.debug(model)
    return InfoResponse(
        model=requested_model,
        max_seq_len=model.get_max_seq_length(),
        embedding_dimension=model.get_sentence_embedding_dimension(),
    )
