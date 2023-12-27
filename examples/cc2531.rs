use anyhow::Context;
use tokio_serial::{SerialPortBuilderExt, SerialPortType};
use z_stack::{NetworkConfig, NodeTable, ZStack};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port_info = tokio_serial::available_ports()?
        .into_iter()
        .find(|p| match p.port_type {
            SerialPortType::UsbPort(ref info) => info
                .product
                .as_ref()
                .map(|p| p.contains("CC2531"))
                .unwrap_or(false),
            _ => false,
        })
        .context("No CC2531 found")?;

    let port_name = port_info.port_name;

    let port = tokio_serial::new(&port_name, 115200).open_native_async()?;

    let node_table = NodeTable::default();

    let zstack = ZStack::new(port, node_table);

    let config = NetworkConfig {
        pan_id: 0x1234,
        channel: 11,
        extended_pan_id: 0x1234567812345678,
    };

    zstack.initialize(config)?;

    loop {
        let event = zstack.wait_for_event().await?;
        println!("{:?}", event);
    }
}
