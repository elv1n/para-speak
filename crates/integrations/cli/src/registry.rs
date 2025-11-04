use anyhow::{anyhow, Result};
use components::{Component, ComponentRef, ExecutionMode};
use dashmap::DashMap;
use log::{error, info};
use std::any::TypeId;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

pub struct ComponentRegistry {
    components: DashMap<TypeId, ComponentRef>,
    components_by_name: DashMap<String, ComponentRef>,
}

pub fn create_default_registry(
    realtime_ring: Option<std::sync::Arc<audio::RingBuffer>>,
) -> Result<RegistryBuilder> {
    use components::{
        AudioComponent, FocusDetectorComponent, TranscriptionObserver,
        SpotifyComponent, TranscriptionHandler,
    };

    info!("Creating default component registry");

    let mut builder = RegistryBuilder::new()
        .register_component(AudioComponent::new())?
        .register_component(SpotifyComponent::new())?
        .register_component(TranscriptionHandler::new())?
        .register_component(FocusDetectorComponent::new())?;

    if let Some(ring) = realtime_ring {
        builder = builder.register_component(TranscriptionObserver::new(ring))?;
    }

    info!(
        "Registry initialized with {} components",
        builder.component_count()
    );
    Ok(builder)
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            components: DashMap::new(),
            components_by_name: DashMap::new(),
        }
    }

    pub fn register<T: Component + 'static>(&self, component: T) -> Result<()> {
        let component_ref: ComponentRef = Arc::new(component);
        let type_id = component_ref.type_id();
        let name = component_ref.name().to_string();

        if self.components.contains_key(&type_id) {
            return Err(anyhow!(
                "Component with type {:?} already registered",
                type_id
            ));
        }

        self.components.insert(type_id, component_ref.clone());
        self.components_by_name.insert(name, component_ref);

        Ok(())
    }

    pub fn get<T: 'static>(&self) -> Option<ComponentRef> {
        let type_id = TypeId::of::<T>();
        self.components.get(&type_id).map(|c| c.clone())
    }

    pub fn get_by_name(&self, name: &str) -> Option<ComponentRef> {
        self.components_by_name.get(name).map(|c| c.clone())
    }

    pub fn initialize_all(&self) -> Result<()> {
        let start = Instant::now();
        let total = self.components.len();

        let parallel_components: Vec<_> = self
            .components
            .iter()
            .filter(|c| c.execution_mode() == ExecutionMode::Parallel)
            .map(|c| c.clone())
            .collect();

        let sequential_components: Vec<_> = self
            .components
            .iter()
            .filter(|c| c.execution_mode() == ExecutionMode::Sequential)
            .map(|c| c.clone())
            .collect();

        let mut failed_components = Vec::new();

        thread::scope(|s| {
            let handles: Vec<_> = parallel_components
                .into_iter()
                .map(|component| {
                    s.spawn(move || {
                        let name = component.name().to_string();
                        if let Err(e) = component.preload() {
                            return Err((name, format!("Preload failed: {}", e)));
                        }

                        if let Err(e) = component.initialize() {
                            return Err((name, format!("Initialize failed: {}", e)));
                        }
                        Ok(name)
                    })
                })
                .collect();

            for handle in handles {
                match handle.join() {
                    Ok(Ok(name)) => {
                        log::debug!("Component '{}' initialized successfully", name);
                    }
                    Ok(Err((name, err))) => {
                        failed_components.push((name, err));
                    }
                    Err(_) => {
                        error!("CRITICAL: Component thread panicked during initialization!");
                        failed_components
                            .push(("Unknown".to_string(), "Thread panicked".to_string()));
                    }
                }
            }
        });

        for component in sequential_components {
            let name = component.name().to_string();
            if let Err(e) = component.preload() {
                failed_components.push((name, format!("Preload failed: {}", e)));
                continue;
            }
            if let Err(e) = component.initialize() {
                failed_components.push((name, format!("Initialize failed: {}", e)));
            }
        }

        if !failed_components.is_empty() {
            for (name, err) in &failed_components {
                error!("{}: {}", name, err);
            }
            return Err(anyhow!(
                "{}/{} components failed to initialize",
                failed_components.len(),
                total
            ));
        }

        info!(
            "{} components initialized in {:.2}ms",
            total,
            start.elapsed().as_secs_f64() * 1000.0
        );
        Ok(())
    }

    pub fn notify_start(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_start())
    }

    pub fn notify_stop(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_stop())
    }

    pub fn notify_pause(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_pause())
    }

    pub fn notify_resume(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_resume())
    }

    pub fn notify_cancel(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_cancel())
    }

    pub fn notify_processing_start(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_processing_start())
    }

    pub fn notify_processing_complete(&self, result: &str) -> Result<()> {
        let result = result.to_string();
        self.broadcast_lifecycle(move |c| c.on_processing_complete(&result))
    }

    pub fn notify_error(&self, error: &str) -> Result<()> {
        let error = error.to_string();
        self.broadcast_lifecycle(move |c| c.on_error(&error))
    }

    pub fn notify_partial_processing_start(&self) -> Result<()> {
        self.broadcast_lifecycle(|c| c.on_partial_processing_start())
    }

    pub fn notify_partial_processing_complete(&self, result: &str) -> Result<()> {
        let result = result.to_string();
        self.broadcast_lifecycle(move |c| c.on_partial_processing_complete(&result))
    }

    fn broadcast_lifecycle<F>(&self, f: F) -> Result<()>
    where
        F: Fn(ComponentRef) -> Result<()> + Clone + Send + Sync,
    {
        let mut errors = Vec::new();

        thread::scope(|s| {
            let handles: Vec<_> = self
                .components
                .iter()
                .map(|component| {
                    let comp = component.clone();
                    let f = f.clone();
                    s.spawn(move || {
                        if let Err(e) = f(comp.clone()) {
                            Some((comp.name().to_string(), e))
                        } else {
                            None
                        }
                    })
                })
                .collect();

            for handle in handles {
                match handle.join() {
                    Ok(Some((name, err))) => {
                        errors.push((name, err));
                    }
                    Ok(None) => {}
                    Err(_) => {
                        error!("CRITICAL: Component thread panicked during lifecycle callback!");
                    }
                }
            }
        });

        if !errors.is_empty() {
            for (name, err) in &errors {
                error!("{}: {}", name, err);
            }
        }

        Ok(())
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RegistryBuilder {
    registry: Arc<ComponentRegistry>,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ComponentRegistry::new()),
        }
    }

    pub fn register_component<T: Component + 'static>(self, component: T) -> Result<Self> {
        let component_name = component.name().to_string();
        self.registry.register(component).map_err(|e| {
            error!("Failed to register {}: {}", component_name, e);
            anyhow!("Component {} registration failed: {}", component_name, e)
        })?;
        Ok(self)
    }

    pub fn with_component<T: Component + 'static>(self, component: T) -> Result<Self> {
        self.registry.register(component)?;
        Ok(self)
    }

    pub fn with_components<I, T>(self, components: I) -> Result<Self>
    where
        I: IntoIterator<Item = T>,
        T: Component + 'static,
    {
        for component in components {
            self.registry.register(component)?;
        }
        Ok(self)
    }

    pub fn component_count(&self) -> usize {
        self.registry.components.len()
    }

    pub fn build(self) -> Result<Arc<ComponentRegistry>> {
        self.registry.initialize_all()?;
        Ok(self.registry)
    }

    pub fn build_without_init(self) -> Arc<ComponentRegistry> {
        self.registry
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
