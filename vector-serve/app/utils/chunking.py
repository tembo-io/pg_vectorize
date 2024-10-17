from typing import List


def recursive_text_chunk(
    text: str,
    chunk_size: int = 1000,
    chunk_overlap: int = 200,
    separators: List[str] = ["\n\n", "\n", " ", ""],
) -> List[str]:
    """Recursively splits text into smaller chunks with overlap."""

    chunks = []
    current_position = 0

    while current_position < len(text):
        next_chunk = None
        for separator in separators:
            next_split = text.rfind(
                separator, current_position, current_position + chunk_size
            )
            if next_split != -1:
                next_chunk = text[current_position:next_split].strip()
                current_position = next_split + len(separator) - chunk_overlap
                break

        if not next_chunk:
            next_chunk = text[current_position : current_position + chunk_size].strip()
            current_position += chunk_size - chunk_overlap

        chunks.append(next_chunk)

    return chunks
