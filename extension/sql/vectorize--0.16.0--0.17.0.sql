-- Manually execute CREATE EXTENSION vectorscale; before running ALTER EXTENSION vectorize UPDATE;
ALTER TYPE vectorize.indexdist ADD VALUE 'vsc_diskann_cosine';
