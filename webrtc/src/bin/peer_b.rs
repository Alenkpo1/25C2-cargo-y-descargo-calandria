use room_rtc::rtc::rtc_peer_connection::{PeerConnectionRole, RtcPeerConnection};
use std::io::{self, BufRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Peer B (Callee)");

    let mut peer_connection = RtcPeerConnection::new(None, PeerConnectionRole::Controlled)?;
    println!(" Local address: {}", peer_connection.local_addr()?);

    println!(" Input the SDP Offer of Peer A:");
    println!(" (Paste the SDP and press Enter twice)\n");

    let offer_sdp = read_multiline_input()?;

    println!(" Processing offer and gathering local candidates...");
    let answer_sdp = peer_connection.process_offer(&offer_sdp)?;

    println!(" ANSWER SDP (copy and send to Peer A):");
    println!("-----------------------------------------");
    print!("{}", answer_sdp);
    println!("-----------------------------------------\n");

    println!(" Press enter when you have pasted the answer into Peer A...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    signal_peer_ready("peer_b_ready");

    println!("\n starting connectivity checks...\n");
    match peer_connection.start_connectivity_checks() {
        Ok(_) => {
            if let Some(remote) = peer_connection.remote_addr()? {
                println!("\n P2P connection established");
                println!("  Local:  {}", peer_connection.local_addr()?);
                println!("  Remote: {}", remote);
                println!("\n ready to stream media over UDP");
            }
        }
        Err(e) => {
            println!("X Error: {}", e);
        }
    }

    println!("\nOK Peer B complete. Press enter...");
    let mut input2 = String::new();
    std::io::stdin().read_line(&mut input2)?;

    Ok(())
}

fn read_multiline_input() -> Result<String, Box<dyn std::error::Error>> {
    let mut input = String::new();
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    while let Some(Ok(line)) = lines.next() {
        if line.trim().is_empty() {
            break;
        }
        input.push_str(&line);
        input.push('\n');
    }

    Ok(input)
}

fn signal_peer_ready(signal_file: &str) {
    let signal_path = format!("{}.signal", signal_file);
    let _ = std::fs::write(&signal_path, "ready");
    println!("   OK Signal sent: {} is ready!", signal_file);
}
