#!/usr/bin/env python3
import sys
import json
import logging
import os
import platform
from pathlib import Path

try:
    import torch
    import soundfile as sf
    import numpy as np
    import io
    import librosa
    import traceback
except ImportError as e:
    print(json.dumps({"error": f"Failed to import required modules: {e}"}), flush=True)
    sys.exit(1)

class ParakeetMLXModel:
    def __init__(self, model_name="mlx-community/parakeet-tdt-0.6b-v3", verbose=False):
        project_root = Path(__file__).parent.parent
        models_dir = project_root / "models"
        os.environ["HF_HOME"] = str(models_dir)
        os.environ["HUGGINGFACE_HUB_CACHE"] = str(models_dir / "hub")
        
        self.model_name = model_name
        self.model = None
        self.verbose = verbose
        
        if verbose:
            logging.basicConfig(level=logging.DEBUG)
        else:
            logging.basicConfig(level=logging.WARNING)
            
        self.logger = logging.getLogger("parakeet_mlx_model")

    def load_model(self):
        if self.model is not None:
            return self.model
        
        self.logger.info(f"Loading {self.model_name}...")
        
        try:
            from parakeet_mlx import from_pretrained
            self.model = from_pretrained(self.model_name)
            self.logger.info("Model loaded successfully")
            return self.model
        except ImportError:
            self.logger.error("parakeet_mlx or mlx not installed. Please run: pip install mlx parakeet-mlx")
            raise
        except Exception as e:
            self.logger.error(f"Error loading model: {str(e)}")
            raise

    def unload(self):
        if self.model is not None:
            del self.model
            self.model = None
            self.logger.info("Model unloaded")

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
            # Import MLX modules for direct processing
            import mlx.core as mx
            from parakeet_mlx import audio as parakeet_audio
            
            # Load audio from bytes into memory
            audio_stream = io.BytesIO(audio_bytes)
            
            audio_data, sample_rate = sf.read(audio_stream)
            
            
            # Ensure audio is mono and at 16kHz
            if len(audio_data.shape) > 1:
                audio_data = audio_data.mean(axis=1)
            
            if sample_rate != 16000:
                # Resample to 16kHz if needed
                audio_data = librosa.resample(audio_data, orig_sr=sample_rate, target_sr=16000)
            
            # Convert to MLX array
            audio_mx = mx.array(audio_data.astype(np.float32))
            
            # Get log-mel features
            features = parakeet_audio.get_logmel(audio_mx, self.model.preprocessor_config)
            
            # Add batch dimension if needed
            if len(features.shape) == 2:
                features = mx.expand_dims(features, axis=0)
            
            # Encode features
            encoded_result = self.model.encoder(features)
            encoded = encoded_result[0] if isinstance(encoded_result, tuple) else encoded_result
            
            # Decode to tokens
            decoded_tokens, _ = self.model.decode(encoded)
            
            # Convert tokens to text
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


class MLDaemon:
    def __init__(self):
        self.model = None
        self.model_type = None
        self.device = None
        logging.basicConfig(level=logging.WARNING)  # Minimal logging for daemon
        
    def load_model(self, model_type="parakeet"):
        try:
            # Check if we're on Apple Silicon for MLX support
            if sys.platform != "darwin" or platform.machine() != "arm64":
                raise RuntimeError("ParakeetMLX is only supported on Apple Silicon (macOS arm64)")
            
            # Set up device
            if torch.backends.mps.is_available():
                self.device = torch.device("mps")
            elif torch.cuda.is_available():
                self.device = torch.device("cuda")
            else:
                self.device = torch.device("cpu")
            
            # Load ParakeetMLX model
            if model_type == "parakeet" or "parakeet" in model_type.lower():
                model_name = "mlx-community/parakeet-tdt-0.6b-v3"
                self.model = ParakeetMLXModel(model_name=model_name, verbose=False)
                self.model.load_model()
                self.model_type = model_type
                
                return f"parakeet-mlx-{model_name.split('/')[-1]}"
            elif model_type == "test":
                # Test mode - use parakeet model but return test name
                model_name = "mlx-community/parakeet-tdt-0.6b-v3"
                self.model = ParakeetMLXModel(model_name=model_name, verbose=False)
                self.model.load_model()
                self.model_type = model_type
                
                return "parakeet-mlx-test"
            else:
                raise RuntimeError(f"Unsupported model type: {model_type}")
                
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
