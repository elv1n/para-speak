use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

#[derive(Error, Debug)]
pub enum PythonSetupError {
    #[error("No suitable Python (3.10+) found")]
    PythonNotFound,
    #[error("Failed to create virtual environment: {0}")]
    VenvCreationFailed(String),
    #[error("Failed to install dependencies: {0}")]
    DependencyInstallFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Command execution failed: {0}")]
    CommandFailed(String),
}

pub struct PythonEnvironment {
    _python_path: PathBuf,
    _venv_path: PathBuf,
    pub venv_python: PathBuf,
}

impl PythonEnvironment {
    pub fn detect_and_setup(project_root: &Path) -> Result<Self, PythonSetupError> {
        let venv_path = project_root.join("python").join("venv");
        
        if venv_path.exists() && venv_path.join("bin").join("python").exists() {
            println!("✅ Virtual environment already exists at: {}", venv_path.display());
            let venv_python = venv_path.join("bin").join("python");
            
            let env = Self {
                _python_path: venv_python.clone(),
                _venv_path: venv_path,
                venv_python,
            };
            
            if env.verify_packages()? {
                println!("✅ All required packages are installed");
                return Ok(env);
            } else {
                println!("📦 Some packages are missing, installing...");
                env.install_requirements(project_root)?;
                return Ok(env);
            }
        }
        
        let python_path = Self::find_python()?;
        println!("🐍 Found Python at: {}", python_path.display());
        
        let env = Self::create_venv(&python_path, project_root)?;
        env.install_requirements(project_root)?;
        
        Ok(env)
    }
    
    fn find_python() -> Result<PathBuf, PythonSetupError> {
        let candidates = vec![
            "/opt/homebrew/bin/python3.13",
            "/opt/homebrew/bin/python3.12",
            "/opt/homebrew/bin/python3.11",
            "/opt/homebrew/bin/python3.10",
            "/usr/local/bin/python3.13",
            "/usr/local/bin/python3.12",
            "/usr/local/bin/python3.11",
            "/usr/local/bin/python3.10",
        ];
        
        for candidate in candidates {
            let path = PathBuf::from(candidate);
            if path.exists() && Self::check_python_version(&path).is_ok() {
                return Ok(path);
            }
        }
        
        for python_name in &["python3.13", "python3.12", "python3.11", "python3.10", "python3"] {
            if let Ok(path) = which::which(python_name) {
                if Self::check_python_version(&path).is_ok() {
                    return Ok(path);
                }
            }
        }
        
        Err(PythonSetupError::PythonNotFound)
    }
    
    fn check_python_version(python_path: &Path) -> Result<(), PythonSetupError> {
        let output = Command::new(python_path)
            .arg("-c")
            .arg("import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
            .output()
            .map_err(|e| PythonSetupError::CommandFailed(e.to_string()))?;
        
        if !output.status.success() {
            return Err(PythonSetupError::CommandFailed("Failed to get Python version".to_string()));
        }
        
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = version.split('.').collect();
        
        if parts.len() == 2 {
            if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if major == 3 && minor >= 10 {
                    return Ok(());
                }
            }
        }
        
        Err(PythonSetupError::CommandFailed(format!("Python version {} is too old", version)))
    }
    
    fn create_venv(python_path: &Path, project_root: &Path) -> Result<Self, PythonSetupError> {
        let venv_path = project_root.join("python").join("venv");
        
        println!("📦 Creating virtual environment...");
        
        let output = Command::new(python_path)
            .arg("-m")
            .arg("venv")
            .arg(&venv_path)
            .output()
            .map_err(|e| PythonSetupError::VenvCreationFailed(e.to_string()))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PythonSetupError::VenvCreationFailed(stderr.to_string()));
        }
        
        let venv_python = venv_path.join("bin").join("python");
        
        Ok(Self {
            _python_path: venv_python.clone(),
            _venv_path: venv_path,
            venv_python,
        })
    }
    
    fn install_requirements(&self, project_root: &Path) -> Result<(), PythonSetupError> {
        let requirements_path = project_root.join("python").join("requirements.txt");
        
        if !requirements_path.exists() {
            return Err(PythonSetupError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("requirements.txt not found at: {}", requirements_path.display())
            )));
        }
        
        println!("📦 Upgrading pip...");
        let output = Command::new(&self.venv_python)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--upgrade")
            .arg("pip")
            .output()
            .map_err(|e| PythonSetupError::DependencyInstallFailed(e.to_string()))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: pip upgrade failed: {}", stderr);
        }
        
        println!("📦 Installing Python dependencies...");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        pb.set_message("Installing packages from requirements.txt...");
        pb.enable_steady_tick(Duration::from_millis(100));
        
        let output = Command::new(&self.venv_python)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("-r")
            .arg(&requirements_path)
            .output()
            .map_err(|e| PythonSetupError::DependencyInstallFailed(e.to_string()))?;
        
        pb.finish_and_clear();
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PythonSetupError::DependencyInstallFailed(stderr.to_string()));
        }
        
        println!("✅ Python dependencies installed successfully");
        Ok(())
    }
    
    fn verify_packages(&self) -> Result<bool, PythonSetupError> {
        let packages = vec![
            ("torch", "PyTorch"),
            ("mlx", "MLX"),
            ("parakeet_mlx", "Parakeet MLX"),
            ("librosa", "Librosa"),
            ("soundfile", "Soundfile"),
            ("numpy", "NumPy"),
        ];
        
        for (package, name) in packages {
            let output = Command::new(&self.venv_python)
                .arg("-c")
                .arg(format!("import {}", package))
                .output()
                .map_err(|e| PythonSetupError::CommandFailed(e.to_string()))?;
            
            if !output.status.success() {
                println!("❌ {} is not installed", name);
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}