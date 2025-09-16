#!/usr/bin/env python3
import sys
import os
import logging
import io
import traceback
from pathlib import Path
from typing import Optional

import numpy as np
import soundfile as sf
import librosa
import torch

from python.models.base_model import BaseTranscriptionModel


class CanaryNemoModel(BaseTranscriptionModel):
    def __init__(self, model_name: str, verbose: bool = False):
        super().__init__(model_name, verbose)
        
        project_root = Path(__file__).parent.parent.parent
        models_dir = project_root / "models"
        os.environ["HF_HOME"] = str(models_dir)
        os.environ["HUGGINGFACE_HUB_CACHE"] = str(models_dir / "hub")
        
        if verbose:
            logging.basicConfig(level=logging.DEBUG)
        else:
            logging.basicConfig(level=logging.WARNING)
            
        self.logger = logging.getLogger("canary_nemo_model")
        self.device = "cuda" if torch.cuda.is_available() else "cpu"
        self.logger.info(f"Using device: {self.device}")

    def load_model(self) -> None:
        if self.model is not None:
            return

        self.logger.info(f"Loading {self.model_name}...")

        try:
            import nemo.collections.asr as nemo_asr

            project_root = Path(__file__).parent.parent.parent
            model_id = self.model_name.replace("/", "--")
            model_path = project_root / "models" / "hub" / f"models--{model_id}" / "snapshots" / "main" / "canary-1b-v2.nemo"

            if not model_path.exists():
                raise FileNotFoundError(f"Model file not found at {model_path}. Please run 'cargo run -p verify-cli' to download the model.")

            self.logger.info(f"Loading model from local file: {model_path}")
            self.model = nemo_asr.models.EncDecCTCModelBPE.restore_from(str(model_path))
            self.model = self.model.to(self.device)
            self.model.eval()

            self.logger.info("Model loaded successfully")
        except ImportError:
            self.logger.error("NeMo not installed. Please install NeMo toolkit")
            raise
        except Exception as e:
            self.logger.error(f"Error loading model: {str(e)}")
            raise

    def unload(self) -> None:
        if self.model is not None:
            del self.model
            self.model = None
            torch.cuda.empty_cache() if self.device == "cuda" else None
            self.logger.info("Model unloaded")

    def get_device_info(self) -> str:
        if self.device == "cuda":
            return f"CUDA ({torch.cuda.get_device_name(0)})"
        else:
            return "CPU"

    def transcribe_raw(self, pcm_bytes: bytes, sample_rate: int = 48000, channels: int = 1) -> str:
        if not pcm_bytes:
            return ""
        
        try:
            audio = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32) / 32768.0

            if channels > 1:
                audio = audio.reshape(-1, channels).mean(axis=1)

            if sample_rate != 16000:
                audio = librosa.resample(audio, orig_sr=sample_rate, target_sr=16000)

            with torch.no_grad():
                transcription = self.model.transcribe(
                    [audio],
                    batch_size=1,
                    source_lang="en",
                    target_lang="en",
                    pnc="yes"
                )
                
                if isinstance(transcription, list) and transcription:
                    return transcription[0]
                elif isinstance(transcription, str):
                    return transcription
                else:
                    return ""
                    
        except Exception as e:
            self.logger.error(f"Transcription error: {str(e)}")
            if self.verbose:
                traceback.print_exc()
            return ""

