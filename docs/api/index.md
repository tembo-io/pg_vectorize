# PG Vectorize API Overview

pg vectorize provides tools for two closely related tasks; vector search and retrieval augmented generation (RAG), and there are APIs dedicated to both of these tasks. Vector search is an important component of RAG and the RAG APIs depend on the vector search APIs. It could be helpful to think of the vector search APIs as lower level than RAG. However, relative to Postgres's APIs, both of these vectorize APIs are very high level.
