# executed during build to "bake" model into the container image
from sentence_transformers import SentenceTransformer

from argparse import ArgumentParser

parser = ArgumentParser()
parser.add_argument("--cache_dir", type=str, default="./models/")

cache_dir = "./models/"

if __name__ == "__main__":
    model = SentenceTransformer("sentence-transformers/all-MiniLM-L12-v2")
    model.save(cache_dir)
