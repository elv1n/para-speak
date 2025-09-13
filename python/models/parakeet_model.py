#!/usr/bin/env python3
import sys
import platform
import os
import logging
import io
import traceback
from pathlib import Path
from typing import Optional

import numpy as np
import soundfile as sf
import librosa

from python.models.base_model import BaseTranscriptionModel


class ParakeetMLXModel(BaseTranscriptionModel):
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
            
        self.logger = logging.getLogger("parakeet_mlx_model")
        
        if sys.platform != "darwin" or platform.machine() != "arm64":
            raise RuntimeError("ParakeetMLX is only supported on Apple Silicon (macOS arm64)")

    def load_model(self) -> None:
        if self.model is not None:
            return
        
        self.logger.info(f"Loading {self.model_name}...")
        
        try:
            from parakeet_mlx import from_pretrained
            self.model = from_pretrained(self.model_name)
            self.logger.info("Model loaded successfully")
        except ImportError:
            self.logger.error("parakeet_mlx or mlx not installed. Please run: pip install mlx parakeet-mlx")
            raise
        except Exception as e:
            self.logger.error(f"Error loading model: {str(e)}")
            raise

    def unload(self) -> None:
        if self.model is not None:
            del self.model
            self.model = None
            self.logger.info("Model unloaded")

    def get_device_info(self) -> str:
        return "Apple Silicon (MLX)"

    def transcribe_raw(self, pcm_bytes: bytes, sample_rate: int = 48000, channels: int = 1) -> str:
        if not pcm_bytes:
            return ""
        
        try:
            import mlx.core as mx
            from parakeet_mlx import audio as parakeet_audio
            
            audio = np.frombuffer(pcm_bytes, dtype=np.int16).astype(np.float32) / 32768.0
            
            if channels > 1:
                audio = audio.reshape(-1, channels).mean(axis=1)
            
            if sample_rate != 16000:
                audio = librosa.resample(audio, orig_sr=sample_rate, target_sr=16000)
            
            audio_mx = mx.array(audio)
            
            features = parakeet_audio.get_logmel(audio_mx, self.model.preprocessor_config)
            
            if len(features.shape) == 2:
                features = mx.expand_dims(features, axis=0)
            
            encoded_result = self.model.encoder(features)
            encoded = encoded_result[0] if isinstance(encoded_result, tuple) else encoded_result
            
            decoded_tokens, _ = self.model.decode(encoded)
            
            if decoded_tokens and decoded_tokens[0]:
                text_parts = []
                for token in decoded_tokens[0]:
                    if hasattr(token, 'text'):
                        text_parts.append(token.text)
                    elif hasattr(token, 'token'):
                        text_parts.append(str(token.token))
                return ''.join(text_parts)
            return ""
                    
        except Exception as e:
            self.logger.error(f"Error during raw transcription: {str(e)}")
            self.logger.debug(f"Full traceback: {traceback.format_exc()}")
            raise RuntimeError(f"Transcription failed: {str(e)}")
    
    def transcribe_from_bytes(self, audio_bytes: bytes) -> str:
        if self.model is None:
            self.load_model()
        
        if not audio_bytes:
            return ""
        
        try:
            import mlx.core as mx
            from parakeet_mlx import audio as parakeet_audio
            
            audio_stream = io.BytesIO(audio_bytes)
            audio_data, sample_rate = sf.read(audio_stream)
            
            if len(audio_data.shape) > 1:
                audio_data = audio_data.mean(axis=1)
            
            if sample_rate != 16000:
                audio_data = librosa.resample(audio_data, orig_sr=sample_rate, target_sr=16000)
            
            audio_mx = mx.array(audio_data.astype(np.float32))
            
            features = parakeet_audio.get_logmel(audio_mx, self.model.preprocessor_config)
            
            if len(features.shape) == 2:
                features = mx.expand_dims(features, axis=0)
            
            encoded_result = self.model.encoder(features)
            encoded = encoded_result[0] if isinstance(encoded_result, tuple) else encoded_result
            
            decoded_tokens, _ = self.model.decode(encoded)
            
            if decoded_tokens and decoded_tokens[0]:
                text_parts = []
                for token in decoded_tokens[0]:
                    if hasattr(token, 'text'):
                        text_parts.append(token.text)
                    elif hasattr(token, 'token'):
                        text_parts.append(str(token.token))
                return ''.join(text_parts)
            return ""
                    
        except Exception as e:
            self.logger.error(f"Error during transcription from bytes: {str(e)}")
            self.logger.debug(f"Full traceback: {traceback.format_exc()}")
            raise