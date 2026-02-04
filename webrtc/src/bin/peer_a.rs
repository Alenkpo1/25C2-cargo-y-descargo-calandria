use room_rtc::rtc::rtc_peer_connection::{PeerConnectionRole, RtcPeerConnection};
use std::io::{self, BufRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Peer A (Caller)");

    let mut peer_connection = RtcPeerConnection::new(None, PeerConnectionRole::Controlling)?;

    let local_addr = peer_connection.local_addr()?;
    println!(" Local address: {}", local_addr);

    println!(" Generating offer...");
    let offer_sdp = peer_connection.create_offer()?;

    println!(" OFFER SDP (copy and send to Peer B):");
    println!("------------------------------------------");
    print!("{}", offer_sdp);
    println!("----------------------------------------\n");

    println!(" input the SDP Answer of peer B:");
    println!("(Paste the SDP and press Enter twice)\n");

    let answer_sdp = read_multiline_input()?;

    println!("Parsing answer and applying remote description...");
    peer_connection.set_remote_description(&answer_sdp)?;

    println!("\n waiting for peer b");
    wait_for_peer_ready("peer_b_ready");

    println!("\nstarting connectivity checks...\n");
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
            println!("X Error in connectivity checks: {}", e);
            println!("   (Maybe there is no real peer listening)");
        }
    }

    println!("\n press enter to exit");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

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

fn wait_for_peer_ready(signal_file: &str) {
    let signal_path = format!("{}.signal", signal_file);

    let _ = std::fs::remove_file(&signal_path);

    println!("   waiting signal from {}...", signal_file);
    while !std::path::Path::new(&signal_path).exists() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let _ = std::fs::remove_file(&signal_path);
    println!("   OK {} is ready", signal_file);
}
