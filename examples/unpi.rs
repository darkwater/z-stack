use anyhow::Context;
use futures::{SinkExt, StreamExt};
use tokio_serial::{SerialPortBuilderExt, SerialPortType};
use z_stack::unpi::{CmdType, Message, Subsystem, UnpiCodec};

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

    let mut codec = tokio_util::codec::Framed::new(port, UnpiCodec);

    codec
        .send(Message {
            cmd_type: CmdType::SyncRequest,
            subsystem: Subsystem::Sys,
            command_id: 0x01,
            data: Default::default(),
        })
        .await?;

    loop {
        while let Some(msg) = codec.next().await {
            let msg = msg?;
            println!("{:?}", msg);
        }
    }
}
