//TODO: make sure all chrome utils as eventually used somewhere
#[allow(dead_code)]
pub(super) mod chrome;

// pub(super) fn get_available_port() -> anyhow::Result<u16> {
//     let listener: TcpListener =
//         TcpListener::bind("127.0.0.1:0").context("Failed to bind to random port")?;
//     let port = listener.local_addr()?.port();
//     drop(listener);
//
//     Ok(port)
// }
