use crate::ComponentError;
use async_trait::async_trait;

#[async_trait]
pub trait Component: Send + Sync {
    fn name(&self) -> &str;
    async fn initialize(&mut self) -> Result<(), ComponentError>;
    async fn execute(&self) -> Result<(), ComponentError>;
    async fn shutdown(&mut self) -> Result<(), ComponentError>;
}