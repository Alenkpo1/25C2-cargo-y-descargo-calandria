//! Connectivity checks for ICE agent.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::time::Duration;

use super::pair::{CandidatePair, CandidatePairState};
use crate::stun::{MessageType, StunMessage};

/// Result of connectivity checks.
pub struct ConnectivityResult {
    pub successful_pairs: usize,
    pub selected_pair: Option<CandidatePair>,
}

/// Perform a connectivity check on a single candidate pair.
/// 
/// Sends a STUN Binding Request and waits for the corresponding response.
pub fn perform_connectivity_check(
    socket: &UdpSocket,
    pair: &CandidatePair,
) -> Result<bool, Box<dyn std::error::Error>> {
    let remote_ip = IpAddr::from_str(&pair.remote_candidate.address)?;
    let remote_addr = SocketAddr::new(remote_ip, pair.remote_candidate.port as u16);

    // Retry up to 3 times with increasing timeout
    for attempt in 0..3 {
        let timeout_ms = 500 + (attempt * 500); // 500ms, 1000ms, 1500ms
        
        let (request, transaction_id) = StunMessage::create_binding_request_with_transaction();
        socket.send_to(&request, remote_addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)))?;

        let mut buf = [0u8; 1024];
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        
        while std::time::Instant::now() < deadline {
            match socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    // Process any STUN message
                    match StunMessage::parse(&buf[..len]) {
                        Ok(response) => match response.message_type {
                            MessageType::BindingResponse => {
                                if response.transaction_id == transaction_id {
                                    socket.set_read_timeout(None)?;
                                    return Ok(true);
                                }
                            }
                            MessageType::BindingRequest => {
                                // Respond to incoming binding requests (important for both peers)
                                let reply = StunMessage::create_binding_success(
                                    response.transaction_id,
                                    addr,
                                );
                                let _ = socket.send_to(&reply, addr);
                            }
                            _ => {}
                        },
                        Err(_) => continue,
                    }
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::TimedOut
                        || err.kind() == std::io::ErrorKind::WouldBlock
                    {
                        break; // Try next attempt
                    }
                    socket.set_read_timeout(None)?;
                    return Err(Box::new(err));
                }
            }
        }
    }
    
    socket.set_read_timeout(None)?;
    Ok(false)
}

/// Sort candidate pairs by priority in descending order.
/// 
/// Uses the ICE priority formula for candidate pairs.
pub fn sort_pairs_by_priority(pairs: &mut Vec<CandidatePair>) {
    let mut pairs_with_priority: Vec<_> = pairs
        .iter()
        .map(|pair| {
            let g = pair.local_candidate.priority as u64;
            let d = pair.remote_candidate.priority as u64;
            let min_priority = g.min(d);
            let max_priority = g.max(d);
            let priority =
                (1u64 << 32) * min_priority + 2 * max_priority + if g > d { 1 } else { 0 };
            (pair.clone(), priority)
        })
        .collect();

    pairs_with_priority.sort_by(|a, b| b.1.cmp(&a.1));

    *pairs = pairs_with_priority
        .into_iter()
        .map(|(pair, _)| pair)
        .collect();
}

/// Calculate the combined priority of a candidate pair.
#[cfg(test)]
pub fn calculate_pair_priority(pair: &CandidatePair) -> u64 {
    let g = pair.local_candidate.priority as u64;
    let d = pair.remote_candidate.priority as u64;

    let min_priority = g.min(d);
    let max_priority = g.max(d);

    (1u64 << 32) * min_priority + 2 * max_priority + if g > d { 1 } else { 0 }
}

/// Run connectivity checks on all candidate pairs.
pub fn run_connectivity_checks(
    socket: &UdpSocket,
    pairs: &mut Vec<CandidatePair>,
    is_controlling: bool,
) -> Result<Option<CandidatePair>, Box<dyn std::error::Error>> {
    println!(" starting connectivity checks...");

    if pairs.is_empty() {
        return Err("No candidate pairs to check".into());
    }

    sort_pairs_by_priority(pairs);

    println!("  trying {} pairs of candidates...", pairs.len());

    let mut successful_pairs = 0;
    let mut selected_pair: Option<CandidatePair> = None;

    let pairs_to_check = pairs.clone();

    for (idx, pair) in pairs_to_check.iter().enumerate() {
        println!(
            "  [{}] Trying: {}:{} → {}:{}",
            idx + 1,
            pair.local_candidate.address,
            pair.local_candidate.port,
            pair.remote_candidate.address,
            pair.remote_candidate.port
        );

        if let Some(p) = pairs.get_mut(idx) {
            p.state = CandidatePairState::InProgress;
        }

        match perform_connectivity_check(socket, pair) {
            Ok(true) => {
                if let Some(p) = pairs.get_mut(idx) {
                    p.state = CandidatePairState::Succeeded;
                }
                successful_pairs += 1;
                println!("    OK Pair works!");

                if selected_pair.is_none() {
                    selected_pair = Some(pair.clone());
                    println!("    Pair selected como candidato principal");
                    if is_controlling {
                        return Ok(selected_pair);
                    }
                } else if is_controlling {
                    return Ok(selected_pair);
                }
            }
            Ok(false) => {
                if let Some(p) = pairs.get_mut(idx) {
                    p.state = CandidatePairState::Failed;
                }
                println!("    X Pair failed");
            }
            Err(e) => {
                if let Some(p) = pairs.get_mut(idx) {
                    p.state = CandidatePairState::Failed;
                }
                println!("    X Error: {}", e);
            }
        }
    }

    if successful_pairs == 0 {
        Err("Neither pair of candidates worked".into())
    } else {
        println!(" {} successful pairs", successful_pairs);
        Ok(selected_pair)
    }
}
