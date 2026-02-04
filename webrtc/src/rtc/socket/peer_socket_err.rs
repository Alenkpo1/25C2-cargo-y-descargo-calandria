use crate::rtc::rtc_const::err_const::{
    BINDING_ERROR, CLONE_ERROR, CONNECT_ERROR, LOCAL_ADDR_ERROR, PEER_SOCKET_ERROR, RECEIVER_ERROR,
    SEND_ERROR,
};
use std::fmt;
use std::io::Error;

#[derive(Debug)]
pub enum PeerSocketErr {
    BindSocketError(Error),
    SetLocalAddrError(Error),
    CloneSocketError(Error),
    NotConnectedSocket,
    ReceiverError(Error),
    SendError(Error),
    PoisonedThread,
    SetRemoteAddrError,
}

impl fmt::Display for PeerSocketErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PeerSocketErr::BindSocketError(err) => {
                writeln!(f, "{}: \"{}\" {}", PEER_SOCKET_ERROR, BINDING_ERROR, err)
            }
            PeerSocketErr::SetLocalAddrError(err) => {
                writeln!(f, "{}: \"{}\" {}", PEER_SOCKET_ERROR, LOCAL_ADDR_ERROR, err)
            }
            PeerSocketErr::CloneSocketError(err) => {
                writeln!(f, "{}: \"{}\" {}", PEER_SOCKET_ERROR, CLONE_ERROR, err)
            }
            PeerSocketErr::NotConnectedSocket => {
                writeln!(f, "{}: \"{}\"", PEER_SOCKET_ERROR, CONNECT_ERROR)
            }
            PeerSocketErr::ReceiverError(err) => {
                writeln!(f, "{}: \"{}\" {}", PEER_SOCKET_ERROR, RECEIVER_ERROR, err)
            }
            PeerSocketErr::SendError(err) => {
                writeln!(f, "{}: \"{}\" {}", PEER_SOCKET_ERROR, SEND_ERROR, err)
            }
            PeerSocketErr::PoisonedThread => writeln!(f, "{}: Poisoned thread", PEER_SOCKET_ERROR),
            PeerSocketErr::SetRemoteAddrError => {
                writeln!(f, "{}: Remote address error ", PEER_SOCKET_ERROR)
            }
        }
    }
}
