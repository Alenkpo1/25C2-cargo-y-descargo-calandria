use crate::protocols::sdp::attribute::Attribute;
use crate::protocols::sdp::media_description::MediaDescription;
use crate::protocols::sdp::origin::Origin;
use crate::protocols::sdp::sdp_error::sdp_error::SdpError;
use crate::protocols::sdp::sdp_version::SdpVersion;
use crate::protocols::sdp::time::Time;

use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct SessionDescription {
    version: SdpVersion,
    origin: Origin,
    time: Time,
    media_description: Vec<MediaDescription>,
    attributes: Vec<Attribute>,
}

impl SessionDescription {
    pub fn new(
        version: SdpVersion,
        origin: Origin,
        time: Time,
        media_description: Vec<MediaDescription>,
        attributes: Vec<Attribute>,
    ) -> SessionDescription {
        SessionDescription {
            version,
            origin,
            time,
            media_description,
            attributes,
        }
    }

    pub fn get_attributes(&self) -> &Vec<Attribute> {
        &self.attributes
    }

    pub fn get_ice_credentials(&self) -> Result<(String, String), String> {
        let mut ice_ufrag: Option<String> = None;
        let mut ice_pwd = None;

        for attr in &self.attributes {
            if let Some(ufrag) = attr.get_ice_ufrag() {
                ice_ufrag = Some(ufrag);
            }

            if let Some(pwd) = attr.get_ice_pwd() {
                ice_pwd = Some(pwd);
            }
        }

        match (ice_ufrag, ice_pwd) {
            (Some(ufrag), Some(pwd)) => Ok((ufrag, pwd)),
            _ => Err("No Ice credentials found in the SDP".to_string()),
        }
    }

    /// extracts all the ICE candidates of the SDP
    pub fn get_ice_candidates(&self) -> Vec<crate::ice::IceCandidate> {
        use crate::ice::{CandidateType, IceCandidate};

        let mut candidates = Vec::new();

        for attr in &self.attributes {
            if let Some(candidate_info) = attr.get_candidate() {
                let candidate_type = match candidate_info.typ.as_str() {
                    "host" => CandidateType::Host,
                    "srflx" => CandidateType::Srflx,
                    "relay" => CandidateType::Relay,
                    _ => CandidateType::Host,
                };

                candidates.push(IceCandidate {
                    name: format!("remote-{}", candidates.len()),
                    address: candidate_info.address.clone(),
                    port: candidate_info.port,
                    candidate_type,
                    priority: candidate_info.priority,
                });
            }
        }

        candidates
    }

    // Devuelve Option<String> con el hash ("AA:BB:CC").
    /// Busca el fingerprint DTLS en los atributos.
    pub fn get_fingerprint(&self) -> Option<String> {
        // 1. Primero buscamos a nivel de sesión (self.attributes)
        for attr in &self.attributes {
            if let Some(fp) = attr.get_fingerprint() {
                return Some(fp);
            }
        }
        /*
        // WIP: 2. Si no lo encuentro, buscamos dentro de cada Media Description
        // (Asumiendo que SessionDescription tiene un campo `media_descriptions`)
        for media in &self.media_descriptions {
            for attr in &media.attributes {
                if let Some(fp) = attr.get_fingerprint() {
                    return Some(fp);
                }
            }
        }
        */

        None
    }
}

impl fmt::Display for SessionDescription {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let media_description_str_vec: Vec<String> = self
            .media_description
            .iter()
            .map(|media_linea| media_linea.to_string())
            .collect();
        let media_description_str = media_description_str_vec.join("");
        let attributes_str_vec: Vec<String> = self
            .attributes
            .iter()
            .map(|attribute_linea| attribute_linea.to_string())
            .collect();
        let attributes_strs = attributes_str_vec.join("");
        write!(
            f,
            "{}{}{}{}{}",
            self.version, self.origin, self.time, media_description_str, attributes_strs
        )
    }
}
impl FromStr for SessionDescription {
    type Err = SdpError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec_sdp: Vec<&str> = s.split('\n').filter(|line| !line.is_empty()).collect();
        if vec_sdp.len() < 5 {
            return Err(SdpError::InvalidSdpFormatLength(vec_sdp.len()));
        }
        let version = SdpVersion::from_str(vec_sdp[0])?;
        let origin = Origin::from_str(vec_sdp[1]).map_err(SdpError::OriginCreationError)?;
        let time = Time::from_str(vec_sdp[2])?;
        let mut vec_media: Vec<MediaDescription> = Vec::new();
        let mut vec_attributes: Vec<Attribute> = Vec::new();
        for line in &vec_sdp[3..] {
            if line.len() < 2 {
                return Err(SdpError::InvalidSdpFormat(line.to_string()));
            }
            match &line[0..2] {
                "m=" => {
                    let media = MediaDescription::from_str(line)
                        .map_err(SdpError::MediaDescriptionCreationError)?;
                    vec_media.push(media);
                }
                "a=" => {
                    let attribute =
                        Attribute::from_str(line).map_err(SdpError::AttributeCreationError)?;
                    vec_attributes.push(attribute);
                }
                _ => {
                    return Err(SdpError::InvalidSdpFormat(line.to_string()));
                }
            }
        }
        Ok(Self::new(version, origin, time, vec_media, vec_attributes))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocols::sdp::address_type::AddressType;
    use crate::protocols::sdp::media_type::MediaType;
    use crate::protocols::sdp::net_type::NetType;
    use crate::protocols::sdp::property_attribute::PropertyAttribute::SendOnly;
    use crate::protocols::sdp::sdp_consts::error_consts::{INVALID_SDP_LENGTH_ERROR, SDP_ERROR};
    use crate::protocols::sdp::sdp_consts::general_consts::{
        EQUAL_SYMBOL, MEDIA_DESCRIPTION_KEY, ORIGIN_KEY,
    };
    use crate::protocols::sdp::transport_protocol::TransportProtocol;
    use crate::protocols::sdp::value_attribute::ValueAttribute::RtpMap;
    fn create_str_origin(
        origin_user: String,
        origin_session_id: i32,
        origin_session_version: i32,
        origin_net_type: NetType,
        origin_address_type: AddressType,
        address: String,
    ) -> String {
        format!(
            "{}{}{} {} {} {} {} {}\n",
            ORIGIN_KEY,
            EQUAL_SYMBOL,
            origin_user,
            origin_session_id,
            origin_session_version,
            origin_net_type,
            origin_address_type,
            address,
        )
    }
    #[test]
    fn test_from_str_session_description_ok() {
        let session_version = SdpVersion::new(0);
        let origin_str = create_str_origin(
            "User1".to_string(),
            123,
            1,
            NetType::In,
            AddressType::IP4,
            "123.0.1.2".to_string(),
        );
        let time: Time = Time::new(10);
        let media_type_value = MediaType::Video;
        let port_value = 4000;
        let transport_protocol_value = TransportProtocol::RtpAvp;
        let fmt_value1 = 50;
        let fmt_value2 = 60;
        let mut fmt: Vec<u8> = Vec::new();
        fmt.push(fmt_value1);
        fmt.push(fmt_value2);
        let media_description_str = format!(
            "{}{}{} {} {} {} {}\n",
            MEDIA_DESCRIPTION_KEY,
            EQUAL_SYMBOL,
            media_type_value,
            port_value,
            transport_protocol_value,
            fmt_value1,
            fmt_value2
        );
        let rtp_map_attribute = RtpMap {
            payload_type: 96,
            encoding_name: "L8".to_string(),
            clock_rate: 8000,
        };
        let attribute1: Attribute = Attribute::new(None, Some(rtp_map_attribute));
        let send_only_attribute = SendOnly;
        let attribute2: Attribute = Attribute::new(Some(send_only_attribute), None);
        let sdp_str = format!(
            "{}{}{}{}{}{}",
            session_version.to_string(),
            origin_str,
            time.to_string(),
            media_description_str.to_string(),
            attribute1.to_string(),
            attribute2.to_string(),
        );
        let sdp = SessionDescription::from_str(&sdp_str).unwrap();
        assert_eq!(sdp.to_string(), sdp_str);
    }
    #[test]
    fn test_from_str_sdp_len_error() {
        let session_version = SdpVersion::new(0);
        let sdp_str = format!("{}", session_version.to_string());
        let sdp_vec: Vec<&str> = sdp_str.trim_end_matches('\n').split('\n').collect();
        let sdp_err = SessionDescription::from_str(&sdp_str).unwrap_err();
        assert_eq!(SdpError::InvalidSdpFormatLength(sdp_vec.len()), sdp_err);
        assert_eq!(
            format!("{}", sdp_err),
            format!(
                "{}: \"{}\" {}\n",
                SDP_ERROR,
                sdp_vec.len(),
                INVALID_SDP_LENGTH_ERROR
            )
        );
    }
}
