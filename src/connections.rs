use std::os::unix::io::RawFd;
use tokio::net::TcpStream;

// use crate::ipc::Resources;
// use std::sync::RwLock;

// use std::collections::HashMap;
// use std::net::TcpStream;

// use unix_ipc::Handle;

// /
//    / pub struct Connections {
//     //pub upstream: RwLock<HashMap<String, TcpStream>>,
//     //pub client: RwLock<HashMap<String, TcpStream>>,
// }

// impl Connections {
//     pub fn new() -> Self {
//         Connections {
//             upstream: RwLock::new(HashMap::new()),
//             client: RwLock::new(HashMap::new()),
//         }
//     }
// }

// impl From<&Connections> for Resources {
//     fn from(conns: &Connections) -> Self {
//         let upstreams = conns.upstream.write().unwrap();
//         let mut pass_upstreams = HashMap::new();
//         upstreams.iter().for_each(|(host, stream)| {
//             pass_upstreams.insert(host.to_string(), Handle::new(stream.try_clone().unwrap()));
//         });

//         let mut pass_clients = HashMap::<String, Handle<TcpStream>>::new();
//         let client = conns.client.write().unwrap();
//         client.iter().for_each(|(host, stream)| {
//             pass_clients.insert(host.to_string(), Handle::new(stream.try_clone().unwrap()));
//         });

//         Resources {
//             upstreams: pass_upstreams,
//             clients: pass_clients,
//         }
//     }
// }
//

#[derive(Debug)]
pub struct SocketPair {
    pub client: TcpStream,
    pub upstream: TcpStream,
}
