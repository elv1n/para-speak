#!/usr/bin/env python3
from abc import ABC, abstractmethod
from typing import Optional


class BaseTranscriptionModel(ABC):
    def __init__(self, model_name: str, verbose: bool = False):
        self.model_name = model_name
        self.verbose = verbose
        self.model = None
    
    @abstractmethod
    def load_model(self) -> None:
        pass
    
    @abstractmethod
    def unload(self) -> None:
        pass
    
    @abstractmethod
    def transcribe_raw(self, pcm_bytes: bytes, sample_rate: int = 48000, channels: int = 1) -> str:
        pass
    
    @abstractmethod
    def transcribe_from_bytes(self, audio_bytes: bytes) -> str:
        pass
    
    @abstractmethod
    def get_device_info(self) -> str:
        pass
    
    def is_loaded(self) -> bool:
        return self.model is not None