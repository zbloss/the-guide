"""Hardware acceleration detection for The Guide.

Detects the best available compute device (CUDA GPU, Apple MPS, or CPU) and
exposes a resolved device string that is consumed by Docling's AcceleratorOptions.

Ollama manages its own GPU detection and memory allocation server-side.
Configure Ollama GPU layers via the OLLAMA_NUM_GPU environment variable before
starting the Ollama server process — it cannot be changed from this Python client.
"""

from __future__ import annotations

import logging
import os

logger = logging.getLogger(__name__)


def detect_device(configured: str = "auto") -> str:
    """Return the compute device to use.

    Args:
        configured: Value from AppConfig.device.  ``"auto"`` triggers
                    detection; any other value is returned as-is.

    Returns:
        One of ``"cuda"``, ``"mps"``, or ``"cpu"`` (or a user-supplied
        string such as ``"cuda:1"``).
    """
    if configured != "auto":
        return configured

    try:
        import torch  # optional — not in project deps

        if torch.cuda.is_available():
            name = torch.cuda.get_device_name(0)
            count = torch.cuda.device_count()
            logger.info("CUDA GPU detected: %s (%d device(s))", name, count)
            return "cuda"

        if torch.backends.mps.is_available():
            logger.info("Apple MPS detected")
            return "mps"

    except ImportError:
        logger.debug("PyTorch not installed — relying on Docling/Ollama auto-detection")

    logger.info("No GPU detected — using CPU")
    return "cpu"


def resolve_num_threads(configured: int = 0) -> int:
    """Return the number of CPU threads for model inference.

    Args:
        configured: Value from AppConfig.num_threads.  ``0`` means auto
                    (use all physical CPU cores reported by the OS).
    """
    if configured > 0:
        return configured
    return os.cpu_count() or 4


def log_hardware_summary(device: str, num_threads: int) -> None:
    """Emit a single INFO line summarising the hardware configuration."""
    logger.info(
        "Hardware acceleration: device=%s  cpu_threads=%d  "
        "(Ollama GPU layers controlled by OLLAMA_NUM_GPU env var)",
        device,
        num_threads,
    )
