pub mod unpi;

use std::fmt::Display;

use tokio_serial::SerialStream;

#[derive(Default)]
pub struct NodeTable {}

pub struct ZStack {
    port: SerialStream,
    node_table: NodeTable,
}

pub struct NetworkConfig {
    pub pan_id: u16,
    pub channel: u8,
    pub extended_pan_id: u64,
}

#[derive(Debug)]
pub enum Error {}

impl Display for Error {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

impl ZStack {
    pub fn new(port: SerialStream, node_table: NodeTable) -> Self {
        Self { port, node_table }
    }

    pub fn initialize(&self, config: NetworkConfig) -> Result<()> {
        Ok(())
    }

    pub async fn wait_for_event(&self) -> Result<()> {
        Ok(())
    }
}
