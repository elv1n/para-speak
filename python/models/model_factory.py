#!/usr/bin/env python3
import logging
from typing import Optional
from python.models.base_model import BaseTranscriptionModel


logger = logging.getLogger("model_factory")


def create_model(model_name: str, verbose: bool = False) -> BaseTranscriptionModel:
    model_name_lower = model_name.lower()
    
    if "mlx-community/parakeet" in model_name_lower or "parakeet" in model_name_lower:
        logger.info(f"Creating ParakeetMLX model for: {model_name}")
        from python.models.parakeet_model import ParakeetMLXModel
        return ParakeetMLXModel(model_name, verbose)
    
    elif "nvidia/canary" in model_name_lower or "canary" in model_name_lower:
        logger.info(f"Creating Canary model for: {model_name}")
        from python.models.canary_model import CanaryNemoModel
        return CanaryNemoModel(model_name, verbose)
    
    else:
        logger.error(f"Unknown or unsupported model: {model_name}")
        raise ValueError(f"Unknown or unsupported model: {model_name}. Supported models: parakeet, canary")


def get_model_type(model_name: str) -> str:
    model_name_lower = model_name.lower()
    
    if "mlx-community/parakeet" in model_name_lower or "parakeet" in model_name_lower:
        return "parakeet-mlx"
    elif "nvidia/canary" in model_name_lower or "canary" in model_name_lower:
        return "canary-nemo"
    else:
        raise ValueError(f"Unknown or unsupported model: {model_name}")