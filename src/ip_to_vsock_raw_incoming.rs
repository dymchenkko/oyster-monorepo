// Summarizing NAT insights
//
// v1: track (src_port, dst_addr, dst_port)
// or any form of stateful NAT for that matter
//
// 1. tracking and assigning ports is a headache
// 2. does not easily scale to many threads and I want to avoid tokio/async if possible
// 3. there should be a fast path
//
// Host does not have any real services running on it
// Therefore, we have a lot of latitude in port assignment
//
// Let us direct map some port ranges directly to skip lookups
// 80, 443, 1024-61439 of enclave -> 80, 443, 1024-61439 of host
//
// Connections to and from the enclave now work directly
// More importantly, we do not need a stateful NAT!
// This means no lookups affecting performance
// This also means the NAT can easily be multi threaded without needing locks
//
// On the enclave, we set ephemeral ports to stay within the same range
// It seems to already be the case in my local system, the max is 60999
//
// Only downside - some ports need to be reserved for the host to use
// 61440-65535 is available for that
// This means the enclave cannot use these ports to reach the internet
// While this should not be an issue in most cases since ephemeral ports do not extend there
// and most applications use ports lower than ephemeral, it _is_ a breaking change

// for incoming packets, we need to _intercept_ them and not just get a copy
// raw sockets do the latter, therefore we go with iptables and nfqueue
// iptables can be used to redirect packets to a nfqueue
// we read it here, do NAT and forward onwards

use nfq::{Queue, Verdict};
use socket2::{SockAddr, Socket};

use raw_proxy::{new_nfq_with_backoff, new_vsock_socket_with_backoff, ProxyError, SocketError};

fn handle_conn(conn_socket: &mut Socket, queue: &mut Queue) -> Result<(), ProxyError> {
    loop {
        let mut msg = queue
            .recv()
            .map_err(SocketError::ReadError)
            .map_err(ProxyError::NfqError)?;

        let buf = msg.get_payload_mut();

        // NAT
        buf[16..20].clone_from_slice(&0x7f000001u32.to_be_bytes());

        // TODO: tcp checksum has to be redone manually, figure out a way to offload
        let ip_header_size = usize::from((buf[0] & 0x0f) * 4);
        let size = buf.len();
        buf[ip_header_size + 16..ip_header_size + 18].clone_from_slice(&[0, 0]);
        let mut csum = 0u32;
        for i in (12..20).step_by(2) {
            let word: u32 = u16::from_be_bytes(buf[i..i + 2].try_into().unwrap()).into();
            csum += word;
        }
        csum += u32::from(u16::from_be_bytes([0, buf[9]]));
        csum += (size - ip_header_size) as u16 as u32;
        for i in (ip_header_size..size - 1).step_by(2) {
            let word: u32 = u16::from_be_bytes(buf[i..i + 2].try_into().unwrap()).into();
            csum += word;
        }
        if size % 2 == 1 {
            csum += u32::from(u16::from_be_bytes([buf[size - 1], 0]));
        }
        csum = (csum >> 16) + (csum & 0xffff);
        csum = (csum >> 16) + (csum & 0xffff);
        csum = !csum;

        buf[ip_header_size + 16..ip_header_size + 18].clone_from_slice(&csum.to_be_bytes()[2..4]);

        // send
        let mut total_sent = 0;
        while total_sent < buf.len() {
            let size = conn_socket
                .send(&buf[total_sent..])
                .map_err(SocketError::WriteError)
                .map_err(ProxyError::VsockError)?;
            total_sent += size;
        }

        // verdicts
        msg.set_verdict(Verdict::Drop);
        queue
            .verdict(msg)
            .map_err(|e| SocketError::VerdictError(Verdict::Drop, e))
            .map_err(ProxyError::NfqError)?;
    }
}

fn main() -> anyhow::Result<()> {
    // nfqueue for incoming packets
    let mut queue = new_nfq_with_backoff(0);

    // get vsock socket
    let vsock_addr = &SockAddr::vsock(88, 1200);
    let mut vsock_socket = new_vsock_socket_with_backoff(vsock_addr);

    loop {
        // do proxying
        // on errors, simply reset the erroring socket
        match handle_conn(&mut vsock_socket, &mut queue) {
            Ok(_) => {
                // should never happen!
                unreachable!("connection handler exited without error");
            }
            Err(err @ ProxyError::NfqError(_)) => {
                println!("{:?}", anyhow::Error::from(err));

                // get nfqueue
                queue = new_nfq_with_backoff(0);
            }
            Err(err @ ProxyError::VsockError(_)) => {
                println!("{:?}", anyhow::Error::from(err));

                // get vsock socket
                vsock_socket = new_vsock_socket_with_backoff(vsock_addr);
            }
            Err(err) => {
                // should never happen!
                unreachable!("connection handler exited with unknown error {err:?}");
            }
        }
    }
}
