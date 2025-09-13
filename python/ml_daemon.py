#!/usr/bin/env python3
import sys
import json
import logging
import os
import platform
from pathlib import Path

try:
    import torch
except ImportError as e:
    print(json.dumps({"error": f"Failed to import required modules: {e}"}), flush=True)
    sys.exit(1)

from python.models.model_factory import create_model
from python.models.base_model import BaseTranscriptionModel




class MLDaemon:
    def __init__(self):
        self.model: BaseTranscriptionModel = None
        self.model_type = None
        self.device = None
        logging.basicConfig(level=logging.WARNING)  # Minimal logging for daemon
        
    def load_model(self, model_name: str):
        """Load model by full name. No defaults or special cases."""
        try:
            # Create and load model using factory
            self.model = create_model(model_name, verbose=False)
            self.model.load_model()
            self.model_type = model_name
            
            # Get device info from model
            device_info = self.model.get_device_info()
            logging.info(f"Model loaded on device: {device_info}")
            
            return f"Loaded: {model_name}"
                
        except Exception as e:
            raise RuntimeError(f"Model loading failed: {str(e)}")
    

    def transcribe_raw(self, pcm_bytes: bytes, sample_rate: int = 48000, channels: int = 1) -> str:
        try:
            if not self.model:
                raise RuntimeError("Model not loaded")
            
            if not pcm_bytes:
                return ""
            
            return self.model.transcribe_raw(pcm_bytes, sample_rate, channels)
            
        except Exception as e:
            raise RuntimeError(f"Transcription failed: {str(e)}")
    
    def transcribe_from_bytes(self, audio_bytes):
        try:
            if not self.model:
                raise RuntimeError("Model not loaded")
            
            if not audio_bytes:
                return ""
            
            return self.model.transcribe_from_bytes(audio_bytes)
            
        except Exception as e:
            raise RuntimeError(f"Transcription from bytes failed: {str(e)}")
    
    def unload_model(self):
        try:
            had_model = self.model is not None
            if self.model is not None:
                if hasattr(self.model, 'unload'):
                    try:
                        self.model.unload()
                    except (Exception, KeyboardInterrupt):
                        # Best-effort unload of model object
                        pass
                try:
                    del self.model
                except (Exception, KeyboardInterrupt):
                    pass
                self.model = None
                self.model_type = None

            # Encourage Python to release memory
            try:
                import gc
                gc.collect()
            except (Exception, KeyboardInterrupt):
                pass

            # Guard PyTorch cache clearing (not strictly needed for MLX)
            try:
                import torch  # noqa: F401
                try:
                    if hasattr(torch, 'cuda') and callable(getattr(torch.cuda, 'empty_cache', None)):
                        if torch.cuda.is_available():
                            torch.cuda.empty_cache()
                except (Exception, KeyboardInterrupt):
                    pass
                try:
                    # mps guards can still raise on some CPU-only builds
                    if (
                        hasattr(torch, 'backends') and hasattr(torch.backends, 'mps') and
                        hasattr(torch.backends.mps, 'is_available') and torch.backends.mps.is_available() and
                        hasattr(torch, 'mps') and hasattr(torch.mps, 'empty_cache')
                    ):
                        try:
                            torch.mps.empty_cache()
                        except (Exception, KeyboardInterrupt):
                            pass
                except (Exception, KeyboardInterrupt):
                    pass
            except (Exception, KeyboardInterrupt):
                pass

            return "Model unloaded successfully" if had_model else "No model to unload"
        except (KeyboardInterrupt, SystemExit) as e:
            # During shutdown, return success even if interrupted
            return f"Model unload interrupted during shutdown (expected): {type(e).__name__}"
        except BaseException as e:
            return f"Unload best-effort with error: {str(e)}"
    
    def cleanup(self):
        # Cleanup should be best-effort and never raise to avoid aborts on shutdown
        try:
            self.unload_model()
            try:
                import gc
                gc.collect()
            except (Exception, KeyboardInterrupt):
                pass
            return "Cleanup complete"
        except (KeyboardInterrupt, SystemExit):
            # Expected during shutdown - not an error
            return "Cleanup interrupted during shutdown (expected)"
        except BaseException as e:
            # Report but do not raise
            return f"Cleanup best-effort with error: {str(e)}"
    
    def run(self):
        try:
            while True:
                line = sys.stdin.readline()
                if not line:
                    break
                
                try:
                    command = json.loads(line.strip())
                    
                    if command.get("action") == "load_model":
                        model_type = command.get("model", "parakeet")
                        response = self.load_model(model_type)
                    
                    
                    elif command.get("action") == "ping":
                        response = {"status": "success", "message": "pong"}
                    
                    elif command.get("action") == "exit":
                        response = {"status": "success", "message": "Exiting daemon"}
                        print(json.dumps(response), flush=True)
                        break
                    
                    else:
                        response = {"status": "error", "message": f"Unknown action: {command.get('action')}"}
                    
                    print(json.dumps(response), flush=True)
                    
                except json.JSONDecodeError as e:
                    error_response = {"status": "error", "message": f"Invalid JSON: {str(e)}"}
                    print(json.dumps(error_response), flush=True)
                
        except KeyboardInterrupt:
            pass
        except Exception as e:
            error_response = {"status": "error", "message": f"Daemon error: {str(e)}"}
            print(json.dumps(error_response), flush=True)

if __name__ == "__main__":
    daemon = MLDaemon()
    daemon.run()
