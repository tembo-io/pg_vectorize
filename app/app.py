import os

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware


from app.routes import router as predict_router

app = FastAPI(title="Tembo-Embedding-Service")
app.include_router(predict_router)

if __name__ == "__main__":
    import uvicorn  # type: ignore
    uvicorn.run("src.app:app", host="0.0.0.0", port=5000, reload=True)
