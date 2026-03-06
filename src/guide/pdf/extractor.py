"""Docling-based PDF extraction.

Converts a PDF (given as bytes or a file path) into a list of page text strings
using Docling's DocumentConverter, which natively handles OCR, tables, and headings
without requiring a vision model for well-formatted PDFs.
"""
from __future__ import annotations

import io
import logging
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

logger = logging.getLogger(__name__)


@dataclass
class PageExtraction:
    page_number: int
    raw_text: str
    headings: list[str] = field(default_factory=list)
    is_dm_only: bool = False


async def extract_document(pdf_bytes: bytes) -> list[PageExtraction]:
    """Convert PDF bytes to structured page extractions using Docling."""
    try:
        from docling.document_converter import DocumentConverter
    except ImportError:
        logger.warning("docling not installed — falling back to plain text extraction")
        return _fallback_extract(pdf_bytes)

    # Disable OCR and table-structure models so Docling doesn't try to
    # download HuggingFace weights (which fail on Windows without Developer
    # Mode due to symlink restrictions).  Well-formatted PDFs (most
    # published campaign books) have embedded text so OCR is unnecessary.
    try:
        from docling.datamodel.pipeline_options import PdfPipelineOptions
        pipeline_options = PdfPipelineOptions()
        pipeline_options.do_ocr = False
        pipeline_options.do_table_structure = False
    except ImportError:
        pipeline_options = None  # type: ignore[assignment]

    with tempfile.NamedTemporaryFile(suffix=".pdf", delete=False) as tmp:
        tmp.write(pdf_bytes)
        tmp_path = Path(tmp.name)

    try:
        if pipeline_options is not None:
            from docling.datamodel.base_models import InputFormat
            from docling.document_converter import PdfFormatOption
            converter = DocumentConverter(
                format_options={
                    InputFormat.PDF: PdfFormatOption(pipeline_options=pipeline_options)
                }
            )
        else:
            converter = DocumentConverter()
        result = converter.convert(str(tmp_path))
        doc = result.document

        # Docling exposes content via export_to_markdown() or iterate pages
        # Use markdown export and split by page markers if available
        pages = _extract_pages_from_docling(doc)
    finally:
        tmp_path.unlink(missing_ok=True)

    return pages


def _extract_pages_from_docling(doc) -> list[PageExtraction]:  # type: ignore[no-untyped-def]
    """Extract per-page text from a Docling Document object."""
    pages: list[PageExtraction] = []

    # Docling Document has a .pages dict with PageItem values
    if hasattr(doc, "pages") and doc.pages:
        for page_no, page in doc.pages.items():
            # Collect text elements on this page
            texts: list[str] = []
            headings: list[str] = []
            for item, _ in doc.iterate_items():
                # Filter by page ref if available
                if hasattr(item, "prov") and item.prov:
                    item_page = item.prov[0].page_no if item.prov else None
                    if item_page != page_no:
                        continue
                label = getattr(item, "label", "")
                text = getattr(item, "text", "") or ""
                if not text:
                    continue
                if "heading" in str(label).lower():
                    level = "##" if "section" in str(label).lower() else "###"
                    headings.append(f"{level} {text}")
                texts.append(text)

            pages.append(PageExtraction(
                page_number=page_no,
                raw_text="\n\n".join(texts),
                headings=headings,
            ))
    else:
        # Fallback: export whole document as single page
        full_text = doc.export_to_markdown()
        pages = [PageExtraction(page_number=1, raw_text=full_text)]

    return pages


def _fallback_extract(pdf_bytes: bytes) -> list[PageExtraction]:
    """Last-resort extraction when Docling is unavailable."""
    return [PageExtraction(
        page_number=1,
        raw_text=f"[PDF content — {len(pdf_bytes)} bytes — Docling not installed]",
    )]
