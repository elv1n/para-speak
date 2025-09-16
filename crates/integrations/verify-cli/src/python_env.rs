use indicatif::{ProgressBar, ProgressStyle};
use ml_utils::get_model_profile;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use thiserror::Error;

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
            println!(
                "✅ Virtual environment already exists at: {}",
                venv_path.display()
            );
            let venv_python = venv_path.join("bin").join("python");

            let env = Self {
                _python_path: venv_python.clone(),
                _venv_path: venv_path,
                venv_python,
            };

            if env.verify_packages()? {
                return Ok(env);
            } else {
                println!("📦 Package conflicts found, reinstalling dependencies...");
                env.install_requirements(project_root)?;
                
                if !env.verify_packages()? {
                    return Err(PythonSetupError::DependencyInstallFailed(
                        "Failed to resolve package conflicts after reinstall".to_string()
                    ));
                }
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

        for python_name in &[
            "python3.13",
            "python3.12",
            "python3.11",
            "python3.10",
            "python3",
        ] {
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
            return Err(PythonSetupError::CommandFailed(
                "Failed to get Python version".to_string(),
            ));
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

        Err(PythonSetupError::CommandFailed(format!(
            "Python version {} is too old",
            version
        )))
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
                format!(
                    "requirements.txt not found at: {}",
                    requirements_path.display()
                ),
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

        println!("📦 Installing build dependencies...");
        let output = Command::new(&self.venv_python)
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("Cython>=3.0.0")
            .arg("wheel")
            .arg("setuptools")
            .output()
            .map_err(|e| PythonSetupError::DependencyInstallFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning: build dependencies installation failed: {}", stderr);
        }

        println!("📦 Installing Python dependencies...");
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
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
            return Err(PythonSetupError::DependencyInstallFailed(
                stderr.to_string(),
            ));
        }

        println!("✅ Python dependencies installed successfully");
        Ok(())
    }

    fn verify_packages(&self) -> Result<bool, PythonSetupError> {
        println!("🔍 Checking for package conflicts...");
        
        let pip_check_output = Command::new(&self.venv_python)
            .arg("-m")
            .arg("pip")
            .arg("check")
            .output()
            .map_err(|e| PythonSetupError::CommandFailed(e.to_string()))?;

        if !pip_check_output.status.success() {
            let stderr = String::from_utf8_lossy(&pip_check_output.stderr);
            if !stderr.is_empty() {
                println!("⚠️  Package dependency conflicts found:");
                println!("   {}", stderr.trim());
            }
            return Ok(false);
        }

        println!("✅ No package conflicts detected");
        Ok(true)
    }






    pub fn ensure_correct_profile(
        &self,
        project_root: &Path,
        model_name: &str,
    ) -> Result<(), PythonSetupError> {
        let needed_profile =
            get_model_profile(model_name).map_err(PythonSetupError::CommandFailed)?;

        let required_packages = match needed_profile {
            "parakeet" => vec!["mlx", "parakeet-mlx"],
            "canary" => vec!["nemo-toolkit", "pytorch-lightning", "hydra-core"],
            _ => vec![],
        };

        if required_packages.is_empty() {
            println!("⚠️  Unknown profile '{}', skipping package verification", needed_profile);
            return Ok(());
        }

        println!("🔍 Verifying required packages for '{}' model...", model_name);

        let mut missing_packages = Vec::new();
        for package in &required_packages {
            let output = Command::new(&self.venv_python)
                .arg("-m")
                .arg("pip")
                .arg("show")
                .arg(package)
                .output()
                .map_err(|e| PythonSetupError::CommandFailed(e.to_string()))?;

            if !output.status.success() {
                missing_packages.push(package);
            }
        }

        if !missing_packages.is_empty() {
            println!("📦 Installing missing packages: {:?}", missing_packages);

            if needed_profile == "canary" {
                println!("⚠️  Canary models are experimental and may require additional setup");
                println!("   Please ensure you have sufficient system resources for NeMo toolkit");
            }

            self.install_requirements(project_root)?;
            println!("✅ Required packages installed for '{}' model", model_name);
        } else {
            println!("✅ All required packages are installed for '{}' model", model_name);
        }

        Ok(())
    }
}
