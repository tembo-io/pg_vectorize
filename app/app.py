import os

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware


from app.routes.transform import router as transform_router

app = FastAPI(title="Tembo-Embedding-Service")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
app.include_router(transform_router)

if __name__ == "__main__":
    import uvicorn  # type: ignore
    uvicorn.run("src.app:app", host="0.0.0.0", port=5000, reload=True)
